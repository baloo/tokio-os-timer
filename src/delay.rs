use futures::{try_ready, Async, Future, Poll};
use std::io;
use std::time::Duration;

use crate::timerfd::TimerFd;

/// A future that completes a specified amount of time from its creation.
///
/// Instances of `Delay` perform no work and complete with `()` once the specified duration has been passed.
pub struct Delay {
    e: Option<tokio_reactor::PollEvented<TimerFd>>,
}

impl Delay {
    /// Create a new `Delay` instance that elapses at now + `delay`.
    pub fn new(delay: Duration) -> io::Result<Self> {
        if delay.as_secs() == 0 && delay.subsec_nanos() == 0 {
            // this would be interpreted as "inactive timer" by timerfd_settime
            return Ok(Self { e: None });
        }

        let tfd = unsafe { libc::timerfd_create(libc::CLOCK_MONOTONIC, libc::TFD_NONBLOCK) };
        if tfd == -1 {
            return Err(io::Error::last_os_error());
        }

        // arm the timer
        let timer = libc::itimerspec {
            it_interval: libc::timespec {
                tv_sec: 0,
                tv_nsec: 0,
            },
            it_value: libc::timespec {
                tv_sec: delay.as_secs() as i64,
                tv_nsec: i64::from(delay.subsec_nanos()),
            },
        };
        let ret = unsafe { libc::timerfd_settime(tfd, 0, &timer, std::ptr::null_mut()) };
        if ret == -1 {
            return Err(io::Error::last_os_error());
        }

        let tfd = TimerFd(tfd);
        let e = tokio_reactor::PollEvented::new(tfd);

        Ok(Self { e: Some(e) })
    }
}

impl Future for Delay {
    type Item = ();
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        if self.e.is_none() {
            return Ok(Async::Ready(()));
        }

        let ready = mio::Ready::readable();
        let _ = try_ready!(self.e.as_mut().unwrap().poll_read_ready(ready));
        // we don't ever _actually_ need to read from a timerfd
        self.e.as_mut().unwrap().clear_read_ready(ready)?;
        Ok(Async::Ready(()))
    }
}
