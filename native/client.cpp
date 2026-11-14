#include "client.h"
#include "mhttp/src/bridge.rs.h"

#include "rust/cxx_async_cppcoro.h"

http_client::http_client(std::shared_ptr<http_context> context)
	: conn(std::make_unique<http_connection>(std::move(context)))
{
}

void http_client::set_custom_hostname(rust::Str hostname)
{
	conn->set_custom_hostname(std::string(hostname));
}

void http_client::set_ssl_preset(const std::shared_ptr<ssl_preset>& preset)
{
	conn->set_ssl_preset(preset);
}

void http_client::set_http2_preset(const std::shared_ptr<http2_preset>& preset)
{
	conn->set_http2_preset(preset);
}

bool http_client::is_open() const
{
	return conn->is_open();
}

RustFutureVoid http_client::connect_direct()
{
	int status = co_await conn->connect();
	if (status < 0)
		co_return produce_status_void_err(status);

	co_return produce_status_void_ok();
}

RustFutureVoid http_client::connect_proxied(const std::shared_ptr<http_proxy>& proxy)
{
	conn->set_proxy(proxy);

	int status = co_await conn->connect();
	if (status < 0)
		co_return produce_status_void_err(status);

	co_return produce_status_void_ok();
}

static RustResponse to_rust(const http_response& res)
{
	RustResponse rv{};
	rv.code = res.code;
	rv.status = res.status;
	rv.headers = res.headers;

	rv.body.reserve(res.body.size());

	for (char c : res.body)
		rv.body.push_back((uint8_t)c);

	return rv;
}

// rust strings only live for the call, copy them before the first suspension

RustFutureResponse http_client::send_http1(rust::Str method, rust::Str path, rust::Str headers, rust::Str body)
{
	http_response res{};

	int status = co_await conn->send_http1(&res, std::string(method), std::string(path), std::string(headers), std::string(body));
	if (status < 0)
		co_return produce_status_response_err(status);

	co_return produce_status_response_ok(to_rust(res));
}

RustFutureResponse http_client::send_http2(rust::Str headers, rust::Str body, int32_t depends_on, int32_t weight, int32_t exclusive)
{
	http_response res{};

	int status = co_await conn->send_http2(&res, std::string(headers), std::string(body), depends_on, weight, exclusive);
	if (status < 0)
		co_return produce_status_response_err(status);

	co_return produce_status_response_ok(to_rust(res));
}

std::unique_ptr<http_client> new_client(rust::Str host, uint16_t port, bool ssl, uint32_t timeout, uint8_t version)
{
	auto ctx = std::make_shared<http_context>(http_context{
		.host = std::string(host),
		.port = port,
		.ssl = ssl,
		.timeout = timeout,
		.version = version == 1 ? http_version::http2 : http_version::http1,
	});

	return std::make_unique<http_client>(std::move(ctx));
}

rust::Str translate_error_code(int32_t status)
{
	return utils::error_message_ccp(status);
}
