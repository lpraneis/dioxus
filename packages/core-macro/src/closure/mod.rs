/// safety: The future will only be polled with exclusive mutable access to all scopes, so dereferencing hooks are safe
#[macro_export]
macro_rules! fut {
    ( move ($( $cap:ident ),*) $fut:expr ) => {
        BorrowedFuture::new(Box::pin(async {
            let $($cap),+ = unsafe{$($cap.upgrade_raw()),*};
            $fut
        }))
    };
    ( $fut:expr ) => {
        BorrowedFuture::new(Box::pin(async {
            $fut
        }))
    };
}

/// safety: The callback will only be called with exclusive mutable access to all scopes, so dereferencing hooks are safe
#[macro_export]
macro_rules! callback {
    ( move ($( $cap:ident ),*) |$($arg:ident)*| $fut:expr ) => {
        BorrowedFuture::new(Box::pin(async {
            let $($cap),+ = unsafe{$($cap.upgrade_raw()),*};
            $fut
        }))
    };
    ( |$($arg:ident)*| ) => {
        BorrowedFuture::new(Box::pin(async {
            $fut
        }))
    };
}
