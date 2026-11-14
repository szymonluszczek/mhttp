#include "parser_state.h"

static int on_status(http_parser* p, const char* at, size_t len)
{
	auto state = (parser_state*)p->data;
	state->response->status.append(at, len);
	return 0;
}

static int on_header_field(http_parser* p, const char* at, size_t len)
{
	auto state = (parser_state*)p->data;

	if (state->prev_header_cb == 2)
		state->response->headers += "\r\n";

	state->response->headers.append(at, len);
	state->prev_header_cb = 1;
	return 0;
}

static int on_header_value(http_parser* p, const char* at, size_t len)
{
	auto state = (parser_state*)p->data;

	if (state->prev_header_cb != 2)
		state->response->headers += ": ";

	state->response->headers.append(at, len);
	state->prev_header_cb = 2;
	return 0;
}

static int on_headers_complete(http_parser* p)
{
	auto state = (parser_state*)p->data;
	state->response->code = p->status_code;

	return state->expect_no_body ? 1 : 0;
}

static int on_body(http_parser* p, const char* at, size_t len)
{
	auto state = (parser_state*)p->data;
	state->response->body.append(at, len);
	return 0;
}

static int on_message_complete(http_parser* p)
{
	auto state = (parser_state*)p->data;
	state->done = true;

	// bytes after the response belong to what comes next (TLS after CONNECT)
	http_parser_pause(p, 1);
	return 0;
}

parser_state::parser_state(std::shared_ptr<http_response> response, bool expect_no_body)
	: expect_no_body(expect_no_body), response(std::move(response))
{
	http_parser_init(&parser, HTTP_RESPONSE);
	parser.data = this;

	http_parser_settings_init(&settings);
	settings.on_status = on_status;
	settings.on_header_field = on_header_field;
	settings.on_header_value = on_header_value;
	settings.on_headers_complete = on_headers_complete;
	settings.on_body = on_body;
	settings.on_message_complete = on_message_complete;
}

int parser_state::on_read(const char* data, size_t data_len)
{
	size_t parsed = http_parser_execute(&parser, &settings, data, data_len);
	if (done)
		return 0;

	return parsed == data_len && HTTP_PARSER_ERRNO(&parser) == HPE_OK ? 0 : -1;
}

int parser_state::on_eof()
{
	if (done)
		return 0;

	http_parser_execute(&parser, &settings, nullptr, 0);
	return done ? 0 : -1;
}
