#include "ssl_manager.h"
#include "utils.h"
#include "mhttp/src/bridge.rs.h"
#include <cstdlib>
#include <fstream>

static void keylog_cb(const SSL* ssl, const char* line)
{
	std::ofstream out(std::getenv("SSLKEYLOGFILE"), std::ios_base::app);
	out << line << "\n";
}

// no cert verification, see README
static ssl_verify_result_t verify_cb(SSL* ssl, uint8_t* out_alert)
{
	return ssl_verify_ok;
}

constexpr uint16_t cert_compression_zstd = 3;

template <uint16_t Algorithm>
static int decompress_cert(SSL* ssl, CRYPTO_BUFFER** out, size_t uncompressed_len, const uint8_t* in, size_t in_len)
{
	uint8_t* data;
	bssl::UniquePtr<CRYPTO_BUFFER> buffer(CRYPTO_BUFFER_alloc(&data, uncompressed_len));
	if (!buffer)
		return 0;

	if (!decompress_certificate(Algorithm, rust::Slice<const uint8_t>(in, in_len), rust::Slice<uint8_t>(data, uncompressed_len)))
		return 0;

	*out = buffer.release();
	return 1;
}

ssl_manager::ssl_manager(std::string host, bool support_h2, std::shared_ptr<ssl_preset> preset)
	: preset(std::move(preset)), host(std::move(host)), support_h2(support_h2)
{
}

void ssl_manager::modify_ctx(SSL_CTX* ctx)
{
	if (std::getenv("SSLKEYLOGFILE") != nullptr)
		SSL_CTX_set_keylog_callback(ctx, keylog_cb);

	SSL_CTX_set_custom_verify(ctx, SSL_VERIFY_PEER, verify_cb);
	SSL_CTX_set_grease_enabled(ctx, preset->enable_grease);

	for (auto alg : preset->cert_compression_algs)
	{
		switch (alg)
		{
		case TLSEXT_cert_compression_zlib:
			SSL_CTX_add_cert_compression_alg(ctx, alg, nullptr, decompress_cert<TLSEXT_cert_compression_zlib>);
			break;
		case TLSEXT_cert_compression_brotli:
			SSL_CTX_add_cert_compression_alg(ctx, alg, nullptr, decompress_cert<TLSEXT_cert_compression_brotli>);
			break;
		case cert_compression_zstd:
			SSL_CTX_add_cert_compression_alg(ctx, alg, nullptr, decompress_cert<cert_compression_zstd>);
			break;
		default:
			critical_log("unknown certificate compression algorithm " + std::to_string(alg) + " ignored");
		}
	}
}

static std::vector<uint8_t> alpn_without_h2(const std::vector<uint8_t>& wire)
{
	std::vector<uint8_t> out;
	for (size_t i = 0; i < wire.size() && i + 1 + wire[i] <= wire.size(); i += 1 + wire[i])
	{
		std::string_view name((const char*)&wire[i + 1], wire[i]);
		if (name != "h2")
			out.insert(out.end(), wire.begin() + i, wire.begin() + i + 1 + wire[i]);
	}
	return out;
}

bool ssl_manager::modify_session(SSL* ssl)
{
	auto& p = *preset;
	bool ok = true;
	auto check = [&](bool result, const char* what) {
		if (!result)
		{
			critical_log(std::string("boringssl rejected the preset's ") + what);
			ok = false;
		}
	};

	check(SSL_set_tlsext_host_name(ssl, host.c_str()), "server name");
	check(SSL_set_min_proto_version(ssl, TLS1_2_VERSION), "minimum version");
	check(SSL_set_max_proto_version(ssl, TLS1_3_VERSION), "maximum version");

	if (!p.cipher_list.empty())
		check(SSL_set_write_cipher_list_override(ssl, p.cipher_list.data(), p.cipher_list.size()), "cipher suites");
	if (!p.supported_versions.empty())
		check(SSL_set_write_supported_versions_override(ssl, p.supported_versions.data(), p.supported_versions.size()), "supported versions");

	if (!p.verify_algorithm_prefs.empty())
		check(SSL_set_verify_algorithm_prefs(ssl, p.verify_algorithm_prefs.data(), p.verify_algorithm_prefs.size()), "signature algorithms");
	if (!p.group_ids.empty())
		check(SSL_set1_group_ids(ssl, p.group_ids.data(), p.group_ids.size()), "groups");

	if (!p.alpn_protos.empty())
	{
		auto alpn = support_h2 ? p.alpn_protos : alpn_without_h2(p.alpn_protos);
		check(SSL_set_alpn_protos(ssl, alpn.data(), alpn.size()) == 0, "alpn");
	}

	if (!p.application_settings.empty() && support_h2)
		check(SSL_add_application_settings(ssl, p.application_settings.data(), p.application_settings.size(), nullptr, 0), "alps");

	if (p.enable_trust_anchors)
		check(SSL_set1_requested_trust_anchors(ssl, p.trust_anchor_ids.data(), p.trust_anchor_ids.size()), "trust anchors");

	if (p.enable_signed_cert_timestamps)
		SSL_enable_signed_cert_timestamps(ssl);

	if (p.enable_ocsp_stapling)
		SSL_enable_ocsp_stapling(ssl);

	SSL_set_enable_ech_grease(ssl, p.enable_ech_grease);
	SSL_set_permute_extensions(ssl, p.permute_extensions);
	SSL_set_alps_use_new_codepoint(ssl, p.enable_new_alps_codepoint);
	SSL_set_signature_algorithm_grease(ssl, p.enable_sigalg_grease);
	SSL_set_client_hello_padding(ssl, p.enable_padding);

	SSL_set_options(ssl, SSL_OP_NO_COMPRESSION | SSL_OP_LEGACY_SERVER_CONNECT | (p.enable_session_ticket ? 0 : SSL_OP_NO_TICKET));
	SSL_set_mode(ssl, SSL_MODE_RELEASE_BUFFERS | SSL_MODE_CBC_RECORD_SPLITTING | SSL_MODE_ENABLE_FALSE_START);

	return ok;
}
