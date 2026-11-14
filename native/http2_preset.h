#ifndef HTTP_LIB_HTTP2_PRESET_H
#define HTTP_LIB_HTTP2_PRESET_H

#include "http2_payload.h"
#include "rust/cxx.h"
#include <memory>
#include <string>
#include <string_view>
#include <vector>

class http2_preset
{
public:
	void settings_add(int32_t id, uint32_t val) { settings.push_back({ id, val }); }
	void set_window_update(int32_t size) { window_size = size; }

	void add_hpack_rule(rust::Str name, uint8_t mode) { hpack_rules.push_back({ std::string(name), mode }); }
	void set_hpack_huffman(uint8_t mode) { hpack_huffman = mode; }
	void set_preface_with_first_request(bool toggle) { preface_with_first_request = toggle; }

	void add_priority_frame(int32_t stream_id, int32_t depends_on, int32_t weight, bool exclusive)
	{
		priority_frames.push_back({ stream_id, depends_on, weight, exclusive });
	}

	uint8_t hpack_flags(std::string_view name) const;

	int32_t window_size = 0;
	std::vector<nghttp2_settings_entry> settings;

	struct hpack_rule { std::string name; uint8_t mode; };
	std::vector<hpack_rule> hpack_rules;
	uint8_t hpack_huffman = 0;

	bool preface_with_first_request = false;

	struct priority_frame { int32_t stream_id, depends_on, weight; bool exclusive; };
	std::vector<priority_frame> priority_frames;
};

std::unique_ptr<http2_preset> new_http2_preset();
std::shared_ptr<http2_preset> share_http2_preset(std::unique_ptr<http2_preset> preset);

std::shared_ptr<http2_preset> get_default_http2_preset();

#endif // HTTP_LIB_HTTP2_PRESET_H
