#ifndef HTTP_LIB_SSL_MANAGER_H
#define HTTP_LIB_SSL_MANAGER_H
#include <openssl/ssl.h>
#include <memory>
#include <string>
#include "ssl_preset.h"

class ssl_manager
{
public:
	ssl_manager(std::string host, bool support_h2, std::shared_ptr<ssl_preset> preset);

	void modify_ctx(SSL_CTX* ctx);

	bool modify_session(SSL* ssl);

	std::shared_ptr<ssl_preset> preset;

private:
	std::string host;
	bool support_h2;
};

#endif // HTTP_LIB_SSL_MANAGER_H
