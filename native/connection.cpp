#include "connection.h"

#include <algorithm>
#include <cassert>
#include <charconv>
#include <cstring>

struct write_req
{
	http_connection* conn;
	std::vector<char> data;
};

static void handle_read(http_connection* conn, ssize_t nread, const char* data)
{
	if (!conn->should_continue())
		return;

	bool http1 = conn->phase == http_phase::proxy || conn->ctx->version == http_version::http1;

	if (nread < 0)
	{
		auto req = conn->request_current_http1;
		if (nread == UV_EOF && http1 && req && req->http1->parser->on_eof() == 0)
			conn->finish_request(req);

		conn->close(nread == UV_EOF ? HTTP_ERR_CLOSED_BY_PEER : HTTP_ERR_READING_FAILED);
		return;
	}

	if (nread == 0)
		return;

	if (!http1)
	{
		auto session = conn->http2_session;
		if (!session)
		{
			conn->close(HTTP_ERR_UNEXPECTED_READ);
			return;
		}

		if (nghttp2_session_mem_recv2(session, (const uint8_t*)data, (size_t)nread) < 0)
		{
			conn->close(HTTP_ERR_PARSING_FAILED);
			return;
		}

		if (!conn->preface_pending && conn->send_http2_buffer(false) < 0)
			return;

		if (!nghttp2_session_want_read(session) && !nghttp2_session_want_write(session))
			conn->close(conn->goaway_received ? HTTP_ERR_H2_GOAWAY_RECEIVED : HTTP_ERR_H2_NO_WANTED_RW);

		return;
	}

	auto req = conn->request_current_http1;
	if (!req)
	{
		conn->close(HTTP_ERR_UNEXPECTED_READ);
		return;
	}

	auto parser = req->http1->parser.get();
	if (parser->on_read(data, (size_t)nread) < 0)
	{
		conn->close(HTTP_ERR_PARSING_FAILED);
		return;
	}

	if (parser->done)
		conn->finish_request(req);
}

static void on_alloc(uv_handle_t* handle, size_t suggested_size, uv_buf_t* buf)
{
	*buf = uv_buf_init(new char[suggested_size], (unsigned int)suggested_size);
}

static void on_read(uv_stream_t* stream, ssize_t nread, const uv_buf_t* buf)
{
	handle_read((http_connection*)stream->loop->data, nread, buf->base);
	delete[] buf->base;
}

static void on_tls_alloc(uvtls_t* tls, size_t suggested_size, uv_buf_t* buf)
{
	*buf = uv_buf_init(new char[suggested_size], (unsigned int)suggested_size);
}

static void on_tls_read(uvtls_t* tls, ssize_t nread, const uv_buf_t* buf)
{
	handle_read((http_connection*)tls->data, nread, buf->base);

	// on error buf still belongs to uvtls
	if (nread > 0)
		delete[] buf->base;
}

static void finish_write(write_req* w, int status)
{
	auto conn = w->conn;
	delete w;

	if (status < 0 && status != UV_ECANCELED)
		conn->close(HTTP_ERR_WRITING_FAILED);
}

static void on_raw_written(uv_write_t* req, int status)
{
	auto w = (write_req*)req->data;
	delete req;
	finish_write(w, status);
}

static void on_tls_written(uvtls_write_t* req, int status)
{
	auto w = (write_req*)req->data;
	delete req;
	finish_write(w, status);
}

static void on_connect(uv_connect_t* req, int status)
{
	auto conn = (http_connection*)req->data;
	if (status == UV_ECANCELED || !conn->should_continue())
		return;

	if (status < 0)
	{
		conn->close(HTTP_ERR_FAILED_TO_CONNECT);
		return;
	}

	if (conn->phase == http_phase::proxy || !conn->ctx->ssl)
	{
		if (uv_read_start((uv_stream_t*)conn->socket, on_alloc, on_read) < 0)
		{
			conn->close(HTTP_ERR_READING_START_FAILED);
			return;
		}
	}

	debug_log("connected");
	conn->state = http_state::connected;
	conn->open_gate(conn->connect_done.get());
}

static void on_getaddrinfo(uv_getaddrinfo_t* req, int status, addrinfo* res)
{
	auto conn = (http_connection*)req->data;
	conn->resolving = false;

	if (status < 0 || !conn->should_continue())
	{
		if (res)
			uv_freeaddrinfo(res);

		if (status < 0 && status != UV_EAI_CANCELED)
			conn->close(HTTP_ERR_DNS_LOOKUP_FAILED);

		return;
	}

	conn->socket = new uv_tcp_t();
	uv_tcp_init(conn->loop, conn->socket);

	status = uv_tcp_connect(&conn->connect_req, conn->socket, res->ai_addr, on_connect);
	uv_freeaddrinfo(res);

	if (status < 0)
		conn->close(HTTP_ERR_FAILED_TO_CONNECT);
}

static void on_tls_connect(uvtls_t* tls, int status)
{
	auto conn = (http_connection*)tls->data;
	if (!conn->should_continue())
		return;

	if (status < 0)
	{
		conn->close(HTTP_ERR_TLS_FAILED_TO_CONNECT);
		return;
	}

	if (uvtls_read_start(tls, on_tls_alloc, on_tls_read) < 0)
	{
		conn->close(HTTP_ERR_TLS_READING_START_FAILED);
		return;
	}

	debug_log("tls connected");
	conn->open_gate(conn->tls_connect_done.get());
}

static void on_wake(uv_async_t* handle)
{
	((http_connection*)handle->data)->drain_posts();
}

static void on_loop_thread(void* arg)
{
	auto conn = (http_connection*)arg;
	uv_run(conn->loop, UV_RUN_DEFAULT);

	conn->drain_posts();
}

static void on_wake_closed(uv_handle_t* handle)
{
	auto conn = (http_connection*)handle->data;
	conn->wake_closed();
	delete (uv_async_t*)handle;
}

static void on_socket_closed(uv_handle_t* handle)
{
	auto conn = (http_connection*)handle->loop->data;
	delete (uv_tcp_t*)handle;
	conn->socket = nullptr;
}

static void on_tls_closed(uvtls_t* tls)
{
	auto conn = (http_connection*)tls->data;

	delete[] tls->alloc_buf.base;

	uvtls_context_destroy(tls->context);
	delete tls->context;
	delete (uv_tcp_t*)tls->stream;
	delete tls;

	conn->tls = nullptr;
	conn->tls_context = nullptr;
	conn->socket = nullptr;
}

static nghttp2_ssize on_http2_send(nghttp2_session* session, const uint8_t* data, size_t length, int flags, void* user_data)
{
	auto conn = (http_connection*)user_data;
	if (!conn->should_continue())
		return NGHTTP2_ERR_CALLBACK_FAILURE;

	if (conn->write((const char*)data, length, conn->ctx->ssl) < 0)
		return NGHTTP2_ERR_CALLBACK_FAILURE;

	return (nghttp2_ssize)length;
}

static nghttp2_ssize on_http2_read_body(nghttp2_session* session, int32_t stream_id, uint8_t* buf, size_t length, uint32_t* data_flags, nghttp2_data_source* source, void* user_data)
{
	auto p = (http2_payload*)source->ptr;
	size_t n = std::min(length, p->body.size() - p->body_sent);

	memcpy(buf, p->body.data() + p->body_sent, n);
	p->body_sent += n;

	if (p->body_sent == p->body.size())
		*data_flags |= NGHTTP2_DATA_FLAG_EOF;

	return (nghttp2_ssize)n;
}

static int on_http2_frame_recv(nghttp2_session* session, const nghttp2_frame* frame, void* user_data)
{
	auto conn = (http_connection*)user_data;

	if (frame->hd.type == NGHTTP2_GOAWAY)
		conn->goaway_received = true;

	return 0;
}

static int on_http2_stream_close(nghttp2_session* session, int32_t stream_id, uint32_t error_code, void* user_data)
{
	auto conn = (http_connection*)user_data;

	auto req = conn->get_request(stream_id);
	if (req && !req->completed && !req->closed_by_error)
		conn->finish_request(req, error_code == NGHTTP2_NO_ERROR ? 0 : HTTP_ERR_H2_RST_STREAM_RECEIVED);

	return 0;
}

static int on_http2_data_chunk_recv(nghttp2_session* session, uint8_t flags, int32_t stream_id, const uint8_t* data, size_t len, void* user_data)
{
	auto conn = (http_connection*)user_data;

	auto req = conn->get_request(stream_id);
	if (req)
		req->response->body.append((const char*)data, len);

	return 0;
}

static int on_http2_header(nghttp2_session* session, const nghttp2_frame* frame, const uint8_t* name, size_t namelen, const uint8_t* value, size_t valuelen, uint8_t flags, void* user_data)
{
	auto conn = (http_connection*)user_data;

	auto req = conn->get_request(frame->hd.stream_id);
	if (!req)
		return 0;

	auto key = std::string_view((const char*)name, namelen);
	auto val = std::string_view((const char*)value, valuelen);
	auto p = req->http2.get();
	auto res = req->response.get();

	if (key == ":status")
	{
		int code = 0;
		auto [end, err] = std::from_chars(val.data(), val.data() + val.size(), code);
		if (err != std::errc() || end != val.data() + val.size())
			return NGHTTP2_ERR_TEMPORAL_CALLBACK_FAILURE;

		p->skipping_informational = code >= 100 && code < 200;
		if (p->skipping_informational)
			return 0;

		res->code = code;
		res->status = http_status_str((http_status)code);
	}
	else if (p->skipping_informational)
	{
		return 0;
	}

	res->headers.append(key);
	res->headers.append(": ");
	res->headers.append(val);
	res->headers.append("\n");

	return 0;
}

http_connection::http_connection(std::shared_ptr<http_context> ctx)
	: ctx(std::move(ctx))
{
}

http_connection::~http_connection()
{
	if (loop_running)
	{
		post([this] {
			close(HTTP_ERR_DESTRUCTOR_CALLED);
			begin_teardown();
		});

		uv_thread_join(&loop_thread);
	}

	if (loop)
	{
		// live handles left, freeing it would be a use-after-free so leak it
		if (uv_loop_close(loop) == 0)
			delete loop;
		else
			critical_log("libuv loop still busy after teardown; leaking it");
	}

	if (http2_session)
		nghttp2_session_del(http2_session);

	for (auto& [id, req] : requests)
		delete req;
}

bool http_connection::is_open() const
{
	return state != http_state::closed && loop_running;
}

bool http_connection::should_continue() const
{
	return state != http_state::closed && !stopping;
}

bool http_connection::in_loop_thread() const
{
	if (!loop_running)
		return false;

	uv_thread_t self = uv_thread_self();
	return uv_thread_equal(&self, &loop_thread);
}

void http_connection::post(std::function<void()> fn)
{
	{
		std::unique_lock lck(posts_m);
		if (accepting_posts)
		{
			posts.push_back(std::move(fn));

			if (wake)
				uv_async_send(wake);
			return;
		}
	}

	fn();
}

void http_connection::drain_posts()
{
	for (;;)
	{
		std::vector<std::function<void()>> batch;
		{
			std::unique_lock lck(posts_m);
			if (posts.empty())
			{
				if (!wake)
					accepting_posts = false;
				return;
			}
			batch.swap(posts);
		}

		for (auto& fn : batch)
			fn();
	}
}

void http_connection::wake_closed()
{
	std::unique_lock lck(posts_m);
	wake = nullptr;
}

void http_connection::begin_teardown()
{
	stopping = true;
	timers->stop_all();

	if (resolving)
		uv_cancel((uv_req_t*)&getaddrinfo_req);

	if (tls)
		uvtls_close(tls, on_tls_closed);
	else if (socket)
		uv_close((uv_handle_t*)socket, on_socket_closed);

	uv_close((uv_handle_t*)wake, on_wake_closed);
}

cppcoro::task<int> http_connection::connect()
{
	if (loop)
		co_return HTTP_ERR_ALREADY_CONNECTED;

	if (!tls_preset) tls_preset = get_default_ssl_preset();
	if (!h2_preset) h2_preset = get_default_http2_preset();

	loop = new uv_loop_t();
	if (uv_loop_init(loop) != 0)
	{
		delete loop;
		loop = nullptr;
		co_return HTTP_ERR_FAILED_TO_CONNECT;
	}
	loop->data = this;

	wake = new uv_async_t();
	uv_async_init(loop, wake, on_wake);
	wake->data = this;

	connect_req.data = this;
	getaddrinfo_req.data = this;
	timers = std::make_unique<timer_manager>(this);
	connect_done = std::make_unique<gate>();
	tls_connect_done = std::make_unique<gate>();

	accepting_posts = true;
	if (uv_thread_create(&loop_thread, on_loop_thread, this) != 0)
	{
		accepting_posts = false;
		co_return HTTP_ERR_FAILED_TO_CONNECT;
	}
	loop_running = true;

	co_await on_loop();

	state = http_state::connecting;
	if (proxy)
		phase = http_phase::proxy;

	auto target_host = proxy ? proxy->get_host() : (custom_hostname.empty() ? ctx->host : custom_hostname);
	auto target_port = std::to_string(proxy ? proxy->get_port() : ctx->port);
	debug_log("connecting to " + target_host + ":" + target_port);

	auto timer = timers->start(ctx->timeout);

	int status = uv_getaddrinfo(loop, &getaddrinfo_req, on_getaddrinfo, target_host.c_str(), target_port.c_str(), nullptr);
	if (status < 0)
	{
		timers->stop(timer);
		co_return close(HTTP_ERR_DNS_LOOKUP_FAILED);
	}

	resolving = true;

	co_await connect_done->wait();
	timers->stop(timer);

	if (!should_continue())
		co_return close_code;

	adjust_socket_settings();

	if (proxy && co_await setup_proxy_connection() < 0)
		co_return close_code;

	if (ctx->version == http_version::http2 && setup_http2_session() < 0)
		co_return close_code;

	if (ctx->ssl && co_await setup_tls_connection() < 0)
		co_return close_code;

	// preface, SETTINGS and WINDOW_UPDATE in one write, or with the first request
	if (ctx->version == http_version::http2)
	{
		if (h2_preset->preface_with_first_request)
			preface_pending = true;
		else if (send_http2_buffer(true) < 0)
			co_return close_code;
	}

	co_return close_code;
}

void http_connection::adjust_socket_settings()
{
	uv_tcp_nodelay(socket, 1);
	uv_tcp_keepalive(socket, 1, std::max(1u, ctx->timeout / 1000));
}

cppcoro::task<int> http_connection::setup_proxy_connection()
{
	auto target_host = custom_hostname.empty() ? ctx->host : custom_hostname;
	auto authority = target_host + ":" + std::to_string(ctx->port);
	debug_log("tunnelling to " + authority);

	std::string headers = "Host: " + authority;
	if (proxy->has_auth())
		headers += "\r\nProxy-Authorization: " + proxy->get_auth();

	http_response res{};
	int status = co_await send_http1_impl(&res, "CONNECT", authority, headers, "", true);
	if (status < 0)
		co_return status;

	if (res.code < 200 || res.code >= 300)
		co_return close(HTTP_ERR_PROXY_ERROR);

	phase = http_phase::target;

	if (ctx->ssl)
		uv_read_stop((uv_stream_t*)socket);

	co_return close_code;
}

int http_connection::start_tls()
{
	tls_context = new uvtls_context_t();
	if (uvtls_context_init(tls_context, 0) < 0)
	{
		delete tls_context;
		tls_context = nullptr;
		return close(HTTP_ERR_TLS_FAILED_TO_CONNECT);
	}

	ssl_mgr = std::make_unique<ssl_manager>(ctx->host, ctx->version == http_version::http2, tls_preset);
	ssl_mgr->modify_ctx((SSL_CTX*)tls_context->impl);

	tls = new uvtls_t{};
	tls->data = this;

	if (uvtls_init(tls, tls_context, (uv_stream_t*)socket) < 0)
	{
		uvtls_context_destroy(tls_context);
		delete tls_context;
		tls_context = nullptr;
		delete tls;
		tls = nullptr;
		return close(HTTP_ERR_TLS_FAILED_TO_CONNECT);
	}

	uvtls_set_hostname(tls, ctx->host.data(), ctx->host.size());

	if (!ssl_mgr->modify_session((SSL*)uvtls_get_ssl(tls)))
		return close(HTTP_ERR_TLS_PRESET_REJECTED);

	if (uvtls_connect(tls, on_tls_connect) < 0)
		return close(HTTP_ERR_TLS_FAILED_TO_CONNECT);

	return 0;
}

cppcoro::task<int> http_connection::setup_tls_connection()
{
	auto timer = timers->start(ctx->timeout);

	if (start_tls() == 0)
		co_await tls_connect_done->wait();

	timers->stop(timer);

	if (!should_continue())
		co_return close_code;

	const uint8_t* alpn_buf = nullptr;
	unsigned alpn_len = 0;
	SSL_get0_alpn_selected((SSL*)uvtls_get_ssl(tls), &alpn_buf, &alpn_len);
	auto selected_alpn = std::string(alpn_buf, alpn_buf + alpn_len);
	debug_log("negotiated alpn: '" + selected_alpn + "'");

	// old HTTP/1.1-only servers may skip ALPN
	if (ctx->version == http_version::http1 && selected_alpn.empty())
		co_return close_code;

	if (selected_alpn != http_version_to_alpn(ctx->version))
		co_return close(HTTP_ERR_UNEXPECTED_ALPN);

	co_return close_code;
}

int http_connection::setup_http2_session()
{
	nghttp2_session_callbacks* cbs;
	if (nghttp2_session_callbacks_new(&cbs) != 0)
		return close(HTTP_ERR_H2_LIBRARY_ERROR);

	nghttp2_session_callbacks_set_send_callback2(cbs, on_http2_send);
	nghttp2_session_callbacks_set_on_frame_recv_callback(cbs, on_http2_frame_recv);
	nghttp2_session_callbacks_set_on_stream_close_callback(cbs, on_http2_stream_close);
	nghttp2_session_callbacks_set_on_data_chunk_recv_callback(cbs, on_http2_data_chunk_recv);
	nghttp2_session_callbacks_set_on_header_callback(cbs, on_http2_header);

	nghttp2_option* opts;
	if (nghttp2_option_new(&opts) != 0)
	{
		nghttp2_session_callbacks_del(cbs);
		return close(HTTP_ERR_H2_LIBRARY_ERROR);
	}

	nghttp2_option_set_no_http_messaging(opts, 1);

	int err = nghttp2_session_client_new2(&http2_session, cbs, this, opts);
	nghttp2_session_callbacks_del(cbs);
	nghttp2_option_del(opts);

	if (err != 0)
		return close(HTTP_ERR_H2_LIBRARY_ERROR);

	if (!h2_preset->settings.empty() &&
		nghttp2_submit_settings(http2_session, NGHTTP2_FLAG_NONE, h2_preset->settings.data(), h2_preset->settings.size()) != 0)
		return close(HTTP_ERR_H2_LIBRARY_ERROR);

	if (h2_preset->window_size > 0 &&
		nghttp2_session_set_local_window_size(http2_session, NGHTTP2_FLAG_NONE, 0, h2_preset->window_size + NGHTTP2_INITIAL_CONNECTION_WINDOW_SIZE) != 0)
		return close(HTTP_ERR_H2_LIBRARY_ERROR);

	for (auto& f : h2_preset->priority_frames)
	{
		nghttp2_priority_spec spec;
		nghttp2_priority_spec_init(&spec, f.depends_on, f.weight, f.exclusive);
		if (nghttp2_submit_priority(http2_session, NGHTTP2_FLAG_NONE, f.stream_id, &spec) != 0)
			return close(HTTP_ERR_H2_LIBRARY_ERROR);
	}

	return 0;
}

cppcoro::task<int> http_connection::send_http1(http_response* response, std::string method, std::string path, std::string headers, std::string body)
{
	return send_http1_impl(response, std::move(method), std::move(path), std::move(headers), std::move(body), false);
}

cppcoro::task<int> http_connection::send_http1_impl(http_response* response, std::string method, std::string path, std::string headers, std::string body, bool connect_request)
{
	co_await on_loop();

	auto lock = co_await http1_lock.scoped_lock_async();

	if (!should_continue())
		co_return state == http_state::closed ? close_code : HTTP_ERR_ALREADY_CLOSED;
	if (state != http_state::connected)
		co_return HTTP_ERR_NOT_READY_YET;
	if (!connect_request && ctx->version != http_version::http1)
		co_return HTTP_ERR_VERSION_MISMATCH;

	auto req = new http_request(this, http_version::http1, connect_request);
	attach_request(req, next_http1_id++);

	auto p = req->http1.get();
	p->method = std::move(method);
	p->path = std::move(path);
	p->headers = std::move(headers);
	p->body = std::move(body);
	request_current_http1 = req;

	auto timer = timers->start(ctx->timeout);
	auto raw = p->stringify();
	write(raw.data(), raw.size(), ctx->ssl && phase == http_phase::target);

	co_await req->done->wait();
	timers->stop(timer);
	request_current_http1 = nullptr;

	int rc = 0;
	if (req->closed_by_error)
		rc = req->error ? req->error : close_code;
	else
		*response = *req->response;

	detach_request(req->id);
	co_return rc;
}

cppcoro::task<int> http_connection::send_http2(http_response* response, std::string headers, std::string body, int32_t prio_stream_id, int32_t prio_weight, int prio_exclusive)
{
	co_await on_loop();

	if (!should_continue())
		co_return state == http_state::closed ? close_code : HTTP_ERR_ALREADY_CLOSED;
	if (state != http_state::connected)
		co_return HTTP_ERR_NOT_READY_YET;
	if (ctx->version != http_version::http2)
		co_return HTTP_ERR_VERSION_MISMATCH;
	if (goaway_received)
		co_return HTTP_ERR_H2_GOAWAY_RECEIVED;

	auto req = new http_request(this, http_version::http2);
	auto p = req->http2.get();
	p->headers = std::move(headers);
	p->body = std::move(body);

	size_t start = 0;
	while (start < p->headers.size())
	{
		size_t end = p->headers.find('\n', start);
		if (end == std::string::npos)
			end = p->headers.size();

		size_t colon = p->headers.find(": ", start);
		if (colon != std::string::npos && colon > start && colon < end)
		{
			nghttp2_nv nv{};
			nv.name = (uint8_t*)p->headers.data() + start;
			nv.namelen = colon - start;
			nv.value = (uint8_t*)p->headers.data() + colon + 2;
			nv.valuelen = end - (colon + 2);
			nv.flags = NGHTTP2_NV_FLAG_NO_COPY_NAME | NGHTTP2_NV_FLAG_NO_COPY_VALUE |
				h2_preset->hpack_flags(std::string_view((const char*)nv.name, nv.namelen));
			p->nva.push_back(nv);
		}

		start = end + 1;
	}

	if (!p->body.empty())
	{
		p->data_provider.source.ptr = p;
		p->data_provider.read_callback = on_http2_read_body;
	}

	if (prio_stream_id > -1)
	{
		nghttp2_priority_spec_init(&p->priority, prio_stream_id, prio_weight, prio_exclusive);
		p->has_priority = true;
	}

	int32_t stream_id = nghttp2_submit_request2(http2_session,
		p->has_priority ? &p->priority : nullptr,
		p->nva.data(), p->nva.size(),
		p->body.empty() ? nullptr : &p->data_provider,
		nullptr);

	if (stream_id < 0)
	{
		delete req;
		co_return HTTP_ERR_H2_LIBRARY_ERROR;
	}

	attach_request(req, stream_id);

	auto timer = timers->start(ctx->timeout, [this, stream_id] {
		auto r = get_request(stream_id);
		if (!r || r->completed || r->closed_by_error)
			return;

		nghttp2_submit_rst_stream(http2_session, NGHTTP2_FLAG_NONE, stream_id, NGHTTP2_CANCEL);
		finish_request(r, HTTP_ERR_TIMEOUT_EXCEEDED);
		send_http2_buffer(false);
	});

	if (preface_pending)
	{
		preface_pending = false;
		send_http2_buffer(true);
	}
	else
	{
		send_http2_buffer(false);
	}

	co_await req->done->wait();
	timers->stop(timer);

	int rc = 0;
	if (req->closed_by_error)
		rc = req->error ? req->error : close_code;
	else
		*response = *req->response;

	detach_request(req->id);
	co_return rc;
}

int http_connection::close(int code)
{
	if (state == http_state::closed)
		return code;

	state = http_state::closed;
	close_code = code;
	debug_log("closing: " + utils::error_message(code));

	if (connect_done) open_gate(connect_done.get());
	if (tls_connect_done) open_gate(tls_connect_done.get());

	for (auto& [id, req] : requests)
	{
		if (!req->completed && !req->closed_by_error)
			finish_request(req, code);
	}

	return code;
}

int http_connection::write(const char* data, size_t len, bool use_tls)
{
	auto w = new write_req{ this, std::vector<char>(data, data + len) };
	auto buf = uv_buf_init(w->data.data(), (unsigned int)len);

	int status;
	if (use_tls)
	{
		auto req = new uvtls_write_t();
		req->data = w;
		status = uvtls_write(req, tls, &buf, 1, on_tls_written);
		if (status < 0)
			delete req;
	}
	else
	{
		auto req = new uv_write_t();
		req->data = w;
		status = uv_write(req, (uv_stream_t*)socket, &buf, 1, on_raw_written);
		if (status < 0)
			delete req;
	}

	if (status < 0)
	{
		delete w;
		return close(HTTP_ERR_WRITING_FAILED);
	}

	return 0;
}

int http_connection::send_http2_buffer(bool coalesce)
{
	if (!should_continue())
		return HTTP_ERR_ALREADY_CLOSED;

	if (!coalesce)
	{
		if (nghttp2_session_send(http2_session) != 0)
			return close(HTTP_ERR_H2_WRITING_FAILED);
		return 0;
	}

	std::string out;
	for (;;)
	{
		const uint8_t* data = nullptr;
		nghttp2_ssize len = nghttp2_session_mem_send2(http2_session, &data);
		if (len < 0)
			return close(HTTP_ERR_H2_WRITING_FAILED);
		if (len == 0)
			break;
		out.append((const char*)data, (size_t)len);
	}

	return out.empty() ? 0 : write(out.data(), out.size(), ctx->ssl);
}

void http_connection::open_gate(gate* g)
{
	post([g] { g->set(); });
}

void http_connection::finish_request(http_request* req, int error)
{
	if (error)
	{
		req->closed_by_error = true;
		req->error = error;
	}
	else
	{
		req->completed = true;
	}

	// by id: the request may be gone when the post runs
	post([this, id = req->id] {
		if (auto r = get_request(id))
			r->done->set();
	});
}

void http_connection::attach_request(http_request* req, int id)
{
	req->id = id;
	requests[id] = req;
}

void http_connection::detach_request(int id)
{
	auto it = requests.find(id);
	if (it == requests.end())
		return;

	delete it->second;
	requests.erase(it);
}

http_request* http_connection::get_request(int id)
{
	auto it = requests.find(id);
	return it == requests.end() ? nullptr : it->second;
}
