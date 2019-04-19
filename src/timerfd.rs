use mio::unix::EventedFd;
use mio::Evented;
use mio::Poll;
use mio::PollOpt;
use mio::Ready;
use mio::Token;
use std::io;
use std::os::unix::io::RawFd;

pub(crate) struct TimerFd(pub RawFd);

impl Evented for TimerFd {
    fn register(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        let evented = EventedFd(&self.0);
        evented.register(poll, token, interest, opts)
    }

    fn reregister(
        &self,
        poll: &Poll,
        token: Token,
        interest: Ready,
        opts: PollOpt,
    ) -> io::Result<()> {
        let evented = EventedFd(&self.0);
        evented.reregister(poll, token, interest, opts)
    }

    fn deregister(&self, poll: &Poll) -> io::Result<()> {
        let evented = EventedFd(&self.0);
        evented.deregister(poll)
    }
}

impl Drop for TimerFd {
    fn drop(&mut self) {
        unsafe { libc::close(self.0) };
    }
}
