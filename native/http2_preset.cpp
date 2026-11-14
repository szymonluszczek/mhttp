#include "http2_preset.h"

uint8_t http2_preset::hpack_flags(std::string_view name) const
{
	uint8_t flags = NGHTTP2_NV_FLAG_NONE;

	if (hpack_huffman == 1)
		flags |= NGHTTP2_NV_FLAG_HUFFMAN;
	else if (hpack_huffman == 2)
		flags |= NGHTTP2_NV_FLAG_NO_HUFFMAN;

	for (auto& rule : hpack_rules)
	{
		bool matches = rule.name == "*" ||
			(rule.name == ":*" && name.starts_with(':')) ||
			rule.name == name;
		if (!matches)
			continue;

		switch (rule.mode)
		{
		case 0: flags |= NGHTTP2_NV_FLAG_INDEX; break;
		case 1: flags |= NGHTTP2_NV_FLAG_WITHOUT_INDEXING; break;
		case 2: flags |= NGHTTP2_NV_FLAG_NO_INDEX; break;
		}
		break;
	}

	return flags;
}

std::unique_ptr<http2_preset> new_http2_preset()
{
	return std::make_unique<http2_preset>();
}

std::shared_ptr<http2_preset> share_http2_preset(std::unique_ptr<http2_preset> preset)
{
	return std::shared_ptr<http2_preset>(std::move(preset));
}

std::shared_ptr<http2_preset> get_default_http2_preset()
{
	static const auto preset = std::make_shared<http2_preset>();
	return preset;
}
