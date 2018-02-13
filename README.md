# spin_lock
A dead simple atomic-bool based spinlock in Rust.

The API is made to exactly match `std::sync::Mutex` to allow for simple `s/sync::Mutex/spin_lock::Lock/` replacement.
