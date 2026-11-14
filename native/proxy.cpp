#include "proxy.h"

http_proxy::http_proxy(std::string host, uint16_t port, std::string auth)
	: host(std::move(host)), port(port), auth(std::move(auth))
{
}

std::shared_ptr<http_proxy> new_proxy(rust::Str host, uint16_t port)
{
	return std::make_shared<http_proxy>(std::string(host), port, std::string());
}

std::shared_ptr<http_proxy> new_proxy_auth(rust::Str host, uint16_t port, rust::Str auth)
{
	return std::make_shared<http_proxy>(std::string(host), port, std::string(auth));
}
