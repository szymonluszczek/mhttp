#ifndef HTTP_LIB_PROXY_H
#define HTTP_LIB_PROXY_H

#include "rust/cxx.h"
#include <cstdint>
#include <memory>
#include <string>

class http_proxy
{
public:
	http_proxy(std::string host, uint16_t port, std::string auth);

	const std::string& get_host() const { return host; }
	uint16_t get_port() const { return port; }
	bool has_auth() const { return !auth.empty(); }
	const std::string& get_auth() const { return auth; }

private:
	std::string host;
	uint16_t port;
	std::string auth;
};

std::shared_ptr<http_proxy> new_proxy(rust::Str host, uint16_t port);
std::shared_ptr<http_proxy> new_proxy_auth(rust::Str host, uint16_t port, rust::Str auth);

#endif // HTTP_LIB_PROXY_H
