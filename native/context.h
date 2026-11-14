#ifndef HTTP_LIB_CONTEXT_H
#define HTTP_LIB_CONTEXT_H
#include "utils.h"
#include <cstdint>
#include <string>

enum class http_state { initialized, connecting, connected, closed };
enum class http_phase { proxy, target };
enum class http_version { http1, http2 };

std::string http_version_to_alpn(http_version ver);

struct http_context
{
	std::string host;
	uint16_t port;
	bool ssl;
	uint32_t timeout;
	http_version version;
};

#endif // HTTP_LIB_CONTEXT_H
