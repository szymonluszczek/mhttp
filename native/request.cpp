#include "request.h"
#include "http1_payload.h"
#include "http2_payload.h"

http_request::http_request(http_connection* conn, http_version version, bool expect_no_body)
	: conn(conn), version(version)
{
	response = std::make_shared<http_response>();
	done = std::make_unique<gate>();

	if (version == http_version::http1)
		http1 = std::make_unique<http1_payload>(this, expect_no_body);
	else
		http2 = std::make_unique<http2_payload>(this);
}

http_request::~http_request() = default;
