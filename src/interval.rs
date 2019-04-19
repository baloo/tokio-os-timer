use futures::{try_ready, Async, Poll, Stream};
use std::io;
use std::time::Duration;

use crate::timerfd::TimerFd;

/// A stream that yields once every time a fixed amount of time elapses.
///
/// Instances of `Interval` perform no work.
pub struct Interval {
    e: Option<tokio_reactor::PollEvented<TimerFd>>,
}

impl Interval {
    /// Create a new `Interval` instance that yields at now + `interval`, and every subsequent
    /// `interval`.
    pub fn new(interval: Duration) -> io::Result<Self> {
        if interval.as_secs() == 0 && interval.subsec_nanos() == 0 {
            // this would be interpreted as "inactive timer" by timerfd_settime
            return Ok(Self {
                e: None,
            });
        }

        let tfd = unsafe { libc::timerfd_create(libc::CLOCK_MONOTONIC, libc::TFD_NONBLOCK) };
        if tfd == -1 {
            return Err(io::Error::last_os_error());
        }

        // arm the timer
        let timer = libc::itimerspec {
            // first expiry
            it_value: libc::timespec {
                tv_sec: interval.as_secs() as i64,
                tv_nsec: i64::from(interval.subsec_nanos()),
            },
            // subsequent expiry intervals
            it_interval: libc::timespec {
                tv_sec: interval.as_secs() as i64,
                tv_nsec: i64::from(interval.subsec_nanos()),
            },
        };
        let ret = unsafe { libc::timerfd_settime(tfd, 0, &timer, std::ptr::null_mut()) };
        if ret == -1 {
            return Err(io::Error::last_os_error());
        }

        let tfd = TimerFd(tfd);
        let e = tokio_reactor::PollEvented::new(tfd);

        Ok(Self {
            e: Some(e),
        })
    }
}

impl Stream for Interval {
    type Item = ();
    type Error = io::Error;
    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        if let Some(ref mut e) = self.e {
            let ready = mio::Ready::readable();
            try_ready!(e.poll_read_ready(ready));

            // do a read to reset
            let mut buf = [0; 8];
            let fd = e.get_ref();
            let ret = unsafe { libc::read(fd.0, buf.as_mut().as_mut_ptr() as *mut _, 8) };
            if ret == -1 {
                let err = io::Error::last_os_error();
                if err.kind() == io::ErrorKind::WouldBlock {
                    e.clear_read_ready(ready)?;
                    return Ok(Async::NotReady);
                }
                return Err(err);
            }
            Ok(Async::Ready(Some(())))
        } else {
            return Ok(Async::Ready(Some(())));
        }

    }
}
