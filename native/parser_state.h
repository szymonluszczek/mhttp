#ifndef HTTP_LIB_PARSER_STATE_H
#define HTTP_LIB_PARSER_STATE_H
#include "http_parser.h"
#include "request.h"
#include <memory>

class parser_state
{
public:
	parser_state(std::shared_ptr<http_response> response, bool expect_no_body);

	int on_read(const char* data, size_t data_len);

	int on_eof();

	bool done = false;
	bool expect_no_body;
	int prev_header_cb = 0;

	std::shared_ptr<http_response> response;

private:
	http_parser parser{};
	http_parser_settings settings{};
};

#endif // HTTP_LIB_PARSER_STATE_H
