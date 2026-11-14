#ifndef HTTP_LIB_GATE_H
#define HTTP_LIB_GATE_H

#include <cppcoro/async_manual_reset_event.hpp>
#include <cppcoro/task.hpp>

class gate
{
public:
    void set();
    bool is_set();

    cppcoro::task<> wait();

private:
    cppcoro::async_manual_reset_event latch;
};

#endif // HTTP_LIB_GATE_H
