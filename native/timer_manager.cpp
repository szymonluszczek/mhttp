#include "timer_manager.h"
#include "connection.h"

struct timer_manager::entry
{
	uv_timer_t handle;
	timer_manager* owner;
	uint32_t id;
	std::function<void()> on_timeout;
};

timer_manager::~timer_manager()
{
	for (auto& [id, e] : timers)
		delete e;
}

uint32_t timer_manager::start(uint32_t millis, std::function<void()> on_timeout)
{
	auto e = new entry{};
	e->owner = this;
	e->id = next_id++;
	e->on_timeout = std::move(on_timeout);

	if (uv_timer_init(conn->loop, &e->handle) != 0)
	{
		delete e;
		return 0;
	}

	e->handle.data = e;
	if (uv_timer_start(&e->handle, on_fire, millis, 0) != 0)
	{
		uv_close((uv_handle_t*)&e->handle, on_closed);
		return 0;
	}

	timers.insert({ e->id, e });
	return e->id;
}

void timer_manager::stop(uint32_t id)
{
	auto it = timers.find(id);
	if (it == timers.end())
		return;

	auto e = it->second;
	timers.erase(it);
	uv_timer_stop(&e->handle);
	uv_close((uv_handle_t*)&e->handle, on_closed);
}

void timer_manager::stop_all()
{
	while (!timers.empty())
		stop(timers.begin()->first);
}

void timer_manager::on_fire(uv_timer_t* handle)
{
	auto e = (entry*)handle->data;
	auto conn = e->owner->conn;
	auto on_timeout = std::move(e->on_timeout);
	e->owner->stop(e->id);

	if (on_timeout)
	{
		on_timeout();
		return;
	}

	debug_log("timeout exceeded, closing the connection");
	conn->close(HTTP_ERR_TIMEOUT_EXCEEDED);
}

void timer_manager::on_closed(uv_handle_t* handle)
{
	delete (entry*)handle->data;
}
