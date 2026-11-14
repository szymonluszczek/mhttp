#ifndef HTTP_LIB_REQUEST_H
#define HTTP_LIB_REQUEST_H
#include <string>
#include <memory>
#include "context.h"
#include "gate.h"

class http_connection;
class http1_payload;
class http2_payload;

struct http_response
{
	std::string status;
	int code = 0;
	std::string headers;
	std::string body;
};

class http_request
{
public:
	http_request(http_connection* conn, http_version version, bool expect_no_body = false);
	~http_request();

	http_connection* conn = nullptr;
	http_version version;
	int id = 0;

	std::unique_ptr<http1_payload> http1;
	std::unique_ptr<http2_payload> http2;

	bool closed_by_error = false;
	int error = 0;

	bool completed = false;

	std::unique_ptr<gate> done;
	std::shared_ptr<http_response> response;
};

#endif // HTTP_LIB_REQUEST_H
