#ifndef HTTP_LIB_SSL_PRESET_H
#define HTTP_LIB_SSL_PRESET_H

#include "rust/cxx.h"
#include <cstdint>
#include <memory>
#include <vector>

class ssl_preset
{
public:
	void set_permute_extensions(bool toggle) { permute_extensions = toggle; }
	void set_enable_grease(bool toggle) { enable_grease = toggle; }
	void set_enable_sigalg_grease(bool toggle) { enable_sigalg_grease = toggle; }
	void set_enable_ech_grease(bool toggle) { enable_ech_grease = toggle; }
	void set_enable_session_ticket(bool toggle) { enable_session_ticket = toggle; }
	void set_enable_ocsp_stapling(bool toggle) { enable_ocsp_stapling = toggle; }
	void set_enable_signed_cert_timestamps(bool toggle) { enable_signed_cert_timestamps = toggle; }
	void set_enable_new_alps_codepoint(bool toggle) { enable_new_alps_codepoint = toggle; }
	void set_enable_padding(bool toggle) { enable_padding = toggle; }

	void set_alpn_protos(rust::Vec<std::uint8_t> data) { alpn_protos.assign(data.begin(), data.end()); }
	void set_application_settings(rust::Vec<std::uint8_t> data) { application_settings.assign(data.begin(), data.end()); }

	void set_trust_anchors(rust::Vec<std::uint8_t> ids)
	{
		enable_trust_anchors = true;
		trust_anchor_ids.assign(ids.begin(), ids.end());
	}

	void add_cert_compression_alg(uint16_t value) { cert_compression_algs.push_back(value); }
	void add_cipher_suite(uint16_t value) { cipher_list.push_back(value); }
	void add_verify_algorithm_pref(uint16_t value) { verify_algorithm_prefs.push_back(value); }
	void add_group_id(uint16_t value) { group_ids.push_back(value); }
	void add_supported_version(uint16_t value) { supported_versions.push_back(value); }

	bool permute_extensions = false;
	bool enable_grease = false;
	bool enable_sigalg_grease = true;
	bool enable_ech_grease = false;
	bool enable_session_ticket = false;
	bool enable_ocsp_stapling = false;
	bool enable_signed_cert_timestamps = false;
	bool enable_new_alps_codepoint = false;
	bool enable_trust_anchors = false;
	bool enable_padding = false;

	std::vector<uint8_t> alpn_protos;
	std::vector<uint8_t> application_settings;
	std::vector<uint8_t> trust_anchor_ids;

	std::vector<uint16_t> cert_compression_algs;
	std::vector<uint16_t> cipher_list;
	std::vector<uint16_t> verify_algorithm_prefs;
	std::vector<uint16_t> group_ids;
	std::vector<uint16_t> supported_versions;
};

std::unique_ptr<ssl_preset> new_ssl_preset();
std::shared_ptr<ssl_preset> share_ssl_preset(std::unique_ptr<ssl_preset> preset);

std::shared_ptr<ssl_preset> get_default_ssl_preset();

#endif // HTTP_LIB_SSL_PRESET_H
