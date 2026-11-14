#include "gate.h"

void gate::set()
{
	latch.set();
}

bool gate::is_set()
{
	return latch.is_set();
}

cppcoro::task<> gate::wait()
{
	co_await latch;
}
