#ifndef HTTP_LIB_UTILS_H
#define HTTP_LIB_UTILS_H

#include <string>

namespace utils
{
	void log_info(const std::string& msg);
	void log_debug(const std::string& msg);
	void log_critical(const std::string& msg);
	const char* error_message_ccp(int code);
	std::string error_message(int code);
};

typedef enum {
	HTTP_ERR_INVALID_ARGUMENT          = -501,
	HTTP_ERR_TIMEOUT_EXCEEDED          = -502,
	HTTP_ERR_DNS_LOOKUP_FAILED         = -503,
	HTTP_ERR_FAILED_TO_CONNECT         = -504,
	HTTP_ERR_READING_START_FAILED      = -505,
	HTTP_ERR_READING_FAILED            = -507,
	HTTP_ERR_PARSING_FAILED            = -508,
	HTTP_ERR_WRITING_FAILED            = -509,
	HTTP_ERR_H2_NO_WANTED_RW           = -510,
	HTTP_ERR_H2_LIBRARY_ERROR          = -512,
	HTTP_ERR_H2_RST_STREAM_RECEIVED    = -513,
	HTTP_ERR_H2_GOAWAY_RECEIVED        = -514,
	HTTP_ERR_H2_WRITING_FAILED         = -515,
	HTTP_ERR_TLS_FAILED_TO_CONNECT     = -516,
	HTTP_ERR_TLS_READING_START_FAILED  = -517,
	HTTP_ERR_VERSION_MISMATCH          = -518,
	HTTP_ERR_ALREADY_CLOSED            = -519,
	HTTP_ERR_NOT_READY_YET             = -520,
	HTTP_ERR_PROXY_ERROR               = -521,
	HTTP_ERR_DESTRUCTOR_CALLED         = -524,
	HTTP_ERR_ALREADY_CONNECTED         = -525,
	HTTP_ERR_UNEXPECTED_READ           = -526,
	HTTP_ERR_UNEXPECTED_ALPN           = -527,
	HTTP_ERR_CLOSED_BY_PEER            = -528,
	HTTP_ERR_TLS_PRESET_REJECTED       = -529,
} http_error;

#ifdef LOGGING_ENABLED
#define debug_log(msg) utils::log_debug(msg)
#else
#define debug_log(msg) ((void)0)
#endif

#define critical_log(msg) utils::log_critical(msg)

#endif // HTTP_LIB_UTILS_H
