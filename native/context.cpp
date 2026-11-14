#include "context.h"

std::string http_version_to_alpn(http_version ver)
{
	return ver == http_version::http2 ? "h2" : "http/1.1";
}
