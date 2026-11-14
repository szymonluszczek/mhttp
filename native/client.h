#ifndef HTTP_LIB_CLIENT_H
#define HTTP_LIB_CLIENT_H

#include "rust/cxx.h"
#include "rust/cxx_async.h"

#include "connection.h"
#include <memory>

struct RustResponse;
struct RustStatusWithVoid;
struct RustStatusWithResponse;

CXXASYNC_DEFINE_FUTURE(rust::Box<RustStatusWithVoid>, RustFutureVoid);
CXXASYNC_DEFINE_FUTURE(rust::Box<RustStatusWithResponse>, RustFutureResponse);

class http_client
{
public:
	explicit http_client(std::shared_ptr<http_context> context);

	void set_custom_hostname(rust::Str hostname);
	void set_ssl_preset(const std::shared_ptr<ssl_preset>& preset);
	void set_http2_preset(const std::shared_ptr<http2_preset>& preset);

	bool is_open() const;

	RustFutureVoid connect_direct();
	RustFutureVoid connect_proxied(const std::shared_ptr<http_proxy>& proxy);

	RustFutureResponse send_http1(rust::Str method, rust::Str path, rust::Str headers, rust::Str body);
	RustFutureResponse send_http2(rust::Str headers, rust::Str body, int32_t depends_on, int32_t weight, int32_t exclusive);

private:
	std::unique_ptr<http_connection> conn;
};

std::unique_ptr<http_client> new_client(rust::Str host, uint16_t port, bool ssl, uint32_t timeout, uint8_t version);
rust::Str translate_error_code(int32_t status);

#endif // HTTP_LIB_CLIENT_H
