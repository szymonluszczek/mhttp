#ifndef HTTP_LIB_TIMER_MANAGER_H
#define HTTP_LIB_TIMER_MANAGER_H
#include <cstdint>
#include <functional>
#include <map>
#include <uv.h>

class http_connection;

class timer_manager
{
public:
	explicit timer_manager(http_connection* conn) : conn(conn) {}
	~timer_manager();

	uint32_t start(uint32_t millis, std::function<void()> on_timeout = {});

	void stop(uint32_t id);

	void stop_all();

private:
	struct entry;
	static void on_fire(uv_timer_t* handle);
	static void on_closed(uv_handle_t* handle);

	http_connection* conn;
	uint32_t next_id = 1;
	std::map<uint32_t, entry*> timers;
};

#endif // HTTP_LIB_TIMER_MANAGER_H
