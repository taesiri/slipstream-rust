#include <stddef.h>
#include "picotls.h"

#if defined(_MSC_VER)
#define LAYOUT_ASSERT_EQ(a, b) typedef char layout_assert_##__LINE__[(a) == (b) ? 1 : -1]
#else
#define LAYOUT_ASSERT_EQ(a, b) _Static_assert((a) == (b), "picotls layout mismatch")
#endif

LAYOUT_ASSERT_EQ(offsetof(ptls_iovec_t, base), 0);
LAYOUT_ASSERT_EQ(offsetof(ptls_iovec_t, len), sizeof(void *));
