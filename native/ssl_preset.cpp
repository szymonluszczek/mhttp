#include "ssl_preset.h"

std::unique_ptr<ssl_preset> new_ssl_preset()
{
	return std::make_unique<ssl_preset>();
}

std::shared_ptr<ssl_preset> share_ssl_preset(std::unique_ptr<ssl_preset> preset)
{
	return std::shared_ptr<ssl_preset>(std::move(preset));
}

std::shared_ptr<ssl_preset> get_default_ssl_preset()
{
	static const auto preset = std::make_shared<ssl_preset>();
	return preset;
}
