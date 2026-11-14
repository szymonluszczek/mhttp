#ifndef HTTP_LIB_HTTP1_PAYLOAD_H
#define HTTP_LIB_HTTP1_PAYLOAD_H
#include <string>
#include <memory>
#include "parser_state.h"

class http_request;

class http1_payload
{
public:
	http1_payload(http_request* parent, bool expect_no_body);
	std::string stringify() const;

	http_request* parent = nullptr;

	std::string method;
	std::string path;
	std::string headers;
	std::string body;

	std::unique_ptr<parser_state> parser;
};

#endif // HTTP_LIB_HTTP1_PAYLOAD_H
