#ifndef HTTP_LIB_CONNECTION_H
#define HTTP_LIB_CONNECTION_H
#include <openssl/ssl.h>
#include <uv.h>
extern "C" {
#include "uvtls/uvtls.h"
}

#include <atomic>
#include <functional>
#include <map>
#include <memory>
#include <mutex>
#include <string>
#include <vector>

#include <cppcoro/async_mutex.hpp>
#include <cppcoro/coroutine.hpp>

#include "gate.h"
#include "proxy.h"
#include "context.h"
#include "request.h"
#include "parser_state.h"
#include "http1_payload.h"
#include "http2_payload.h"
#include "ssl_manager.h"
#include "timer_manager.h"
#include "http2_preset.h"

class http_connection
{
public:
	explicit http_connection(std::shared_ptr<http_context> ctx);
	~http_connection();

	void set_proxy(std::shared_ptr<http_proxy> proxy) { this->proxy = std::move(proxy); }
	void set_ssl_preset(std::shared_ptr<ssl_preset> preset) { tls_preset = std::move(preset); }
	void set_http2_preset(std::shared_ptr<http2_preset> preset) { h2_preset = std::move(preset); }
	void set_custom_hostname(std::string hostname) { custom_hostname = std::move(hostname); }

	cppcoro::task<int> connect();
	cppcoro::task<int> send_http1(http_response* response, std::string method, std::string path, std::string headers, std::string body);
	cppcoro::task<int> send_http2(http_response* response, std::string headers, std::string body, int32_t prio_stream_id, int32_t prio_weight, int prio_exclusive);

	bool is_open() const;

	void post(std::function<void()> fn);

	int close(int code);
	bool should_continue() const;

	int write(const char* data, size_t len, bool use_tls);
	int send_http2_buffer(bool coalesce);

	http_request* get_request(int id);
	void finish_request(http_request* req, int error = 0);
	void open_gate(gate* g);

	void begin_teardown();
	void drain_posts();
	void wake_closed();

	std::shared_ptr<http_context> ctx;
	std::shared_ptr<http_proxy> proxy;
	std::shared_ptr<ssl_preset> tls_preset;
	std::shared_ptr<http2_preset> h2_preset;
	std::string custom_hostname;

	std::atomic<http_state> state = http_state::initialized;
	http_phase phase = http_phase::target;
	int close_code = 0;
	bool stopping = false;

	uv_loop_t* loop = nullptr;
	uv_async_t* wake = nullptr;
	uv_tcp_t* socket = nullptr;
	uv_connect_t connect_req{};
	uv_getaddrinfo_t getaddrinfo_req{};
	bool resolving = false;

	uvtls_t* tls = nullptr;
	uvtls_context_t* tls_context = nullptr;
	std::unique_ptr<ssl_manager> ssl_mgr;

	std::unique_ptr<gate> connect_done;
	std::unique_ptr<gate> tls_connect_done;

	std::map<int, http_request*> requests;
	int next_http1_id = 1 << 30;
	http_request* request_current_http1 = nullptr;
	cppcoro::async_mutex http1_lock;

	nghttp2_session* http2_session = nullptr;
	bool goaway_received = false;
	bool preface_pending = false;

	std::unique_ptr<timer_manager> timers;

private:
	struct loop_hop
	{
		http_connection* conn;
		bool await_ready() const noexcept { return conn->in_loop_thread(); }
		void await_suspend(cppcoro::coroutine_handle<> h) { conn->post([h] { h.resume(); }); }
		void await_resume() const noexcept {}
	};

	loop_hop on_loop() { return loop_hop{ this }; }
	bool in_loop_thread() const;

	cppcoro::task<int> send_http1_impl(http_response* response, std::string method, std::string path, std::string headers, std::string body, bool connect_request);
	cppcoro::task<int> setup_proxy_connection();
	cppcoro::task<int> setup_tls_connection();
	int start_tls();
	int setup_http2_session();
	void adjust_socket_settings();

	void attach_request(http_request* req, int id);
	void detach_request(int id);

	bool loop_running = false;
	uv_thread_t loop_thread{};

	std::mutex posts_m;
	std::vector<std::function<void()>> posts;
	bool accepting_posts = false;
};

#endif // HTTP_LIB_CONNECTION_H
