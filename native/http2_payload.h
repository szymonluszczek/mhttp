#ifndef HTTP_LIB_HTTP2_PAYLOAD_H
#define HTTP_LIB_HTTP2_PAYLOAD_H

#define NGHTTP2_STATICLIB 1

// msvc has no ssize_t
#if defined(_MSC_VER) && !defined(_SSIZE_T_DEFINED)
#include <stdint.h>
typedef intptr_t ssize_t;
#define _SSIZE_T_DEFINED
#endif

#include <nghttp2/nghttp2.h>

#include <string>
#include <vector>

class http_request;

class http2_payload
{
public:
	explicit http2_payload(http_request* parent) : parent(parent) {}

	http_request* parent = nullptr;

	std::string headers;
	std::vector<nghttp2_nv> nva;
	std::string body;
	size_t body_sent = 0;

	nghttp2_data_provider2 data_provider{};
	bool has_priority = false;
	nghttp2_priority_spec priority{};

	bool skipping_informational = false;
};

#endif // HTTP_LIB_HTTP2_PAYLOAD_H
