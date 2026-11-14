#include "http1_payload.h"
#include "request.h"
#include <sstream>

http1_payload::http1_payload(http_request* parent, bool expect_no_body)
	: parent(parent)
{
	parser = std::make_unique<parser_state>(parent->response, expect_no_body);
}

std::string http1_payload::stringify() const
{
	std::stringstream ss;
	ss << method << " " << path << " HTTP/1.1\r\n";
	if (!headers.empty())
		ss << headers << "\r\n";
	ss << "\r\n";
	ss << body;
	return ss.str();
}
