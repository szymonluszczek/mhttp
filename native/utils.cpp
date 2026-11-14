#include "utils.h"
#include <iostream>
#include <mutex>

static std::mutex cout_mutex;

namespace utils
{
	void log_info(const std::string& msg)
	{
		std::unique_lock lck(cout_mutex);
		std::cout << msg << std::endl;
	}

	void log_debug(const std::string& msg)
	{
		std::unique_lock lck(cout_mutex);
		std::cout << "[DEBUG] " << msg << std::endl;
	}

	void log_critical(const std::string& msg)
	{
		std::unique_lock lck(cout_mutex);
		std::cerr << "[CRITICAL] " << msg << std::endl;
	}

	const char* error_message_ccp(int code)
	{
		switch (code)
		{
		case 0:                                 return "success";
		case HTTP_ERR_INVALID_ARGUMENT:         return "invalid argument";
		case HTTP_ERR_TIMEOUT_EXCEEDED:         return "timeout exceeded";
		case HTTP_ERR_DNS_LOOKUP_FAILED:        return "dns lookup failed";
		case HTTP_ERR_FAILED_TO_CONNECT:        return "failed to connect";
		case HTTP_ERR_READING_START_FAILED:     return "reading start failed";
		case HTTP_ERR_READING_FAILED:           return "reading failed";
		case HTTP_ERR_PARSING_FAILED:           return "parsing failed";
		case HTTP_ERR_WRITING_FAILED:           return "writing failed";
		case HTTP_ERR_H2_NO_WANTED_RW:          return "h2: no wanted read/write";
		case HTTP_ERR_H2_LIBRARY_ERROR:         return "h2: library error";
		case HTTP_ERR_H2_RST_STREAM_RECEIVED:   return "h2: stream reset by peer";
		case HTTP_ERR_H2_GOAWAY_RECEIVED:       return "h2: goaway received";
		case HTTP_ERR_H2_WRITING_FAILED:        return "h2: writing failed";
		case HTTP_ERR_TLS_FAILED_TO_CONNECT:    return "tls: failed to connect";
		case HTTP_ERR_TLS_READING_START_FAILED: return "tls: reading start failed";
		case HTTP_ERR_VERSION_MISMATCH:         return "protocol version mismatch";
		case HTTP_ERR_ALREADY_CLOSED:           return "connection already closed";
		case HTTP_ERR_NOT_READY_YET:            return "connection not ready yet";
		case HTTP_ERR_PROXY_ERROR:              return "proxy error";
		case HTTP_ERR_DESTRUCTOR_CALLED:        return "connection dropped";
		case HTTP_ERR_ALREADY_CONNECTED:        return "already connected";
		case HTTP_ERR_UNEXPECTED_READ:          return "unexpected read";
		case HTTP_ERR_UNEXPECTED_ALPN:          return "unexpected alpn negotiated";
		case HTTP_ERR_CLOSED_BY_PEER:           return "connection closed by peer";
		case HTTP_ERR_TLS_PRESET_REJECTED:      return "tls: preset rejected by boringssl";
		default:                                return "unknown error code";
		}
	}

	std::string error_message(int code)
	{
		return error_message_ccp(code);
	}
}
