use std::io;
use std::mem::MaybeUninit;
use std::os::unix::io::RawFd;

macro_rules! try_os {
    ($e:expr) => {
        loop {
            let ret = $e;
            if ret == -1 {
                let err = ::std::io::Error::last_os_error();
                if err.kind() == ::std::io::ErrorKind::Interrupted {
                    continue;
                }
                return Err(err);
            } else {
                break ret;
            }
        }
    };
}

pub fn getfl(fd: RawFd) -> io::Result<libc::c_int> {
    Ok(try_os!(unsafe { libc::fcntl(fd, libc::F_GETFL) }))
}

pub fn setfl(fd: RawFd, flags: libc::c_int) -> io::Result<()> {
    try_os!(unsafe { libc::fcntl(fd, libc::F_SETFL, flags) });
    Ok(())
}

pub fn close(fd: RawFd) -> io::Result<()> {
    try_os!(unsafe { libc::close(fd) });
    Ok(())
}

pub fn read(fd: RawFd, buffer: &mut [MaybeUninit<u8>]) -> io::Result<usize> {
    Ok(
        try_os!(unsafe { libc::read(fd, buffer.as_mut_ptr() as *mut libc::c_void, buffer.len()) })
            as usize,
    )
}

pub fn write(fd: RawFd, buffer: &[u8]) -> io::Result<usize> {
    Ok(
        try_os!(unsafe { libc::write(fd, buffer.as_ptr() as *mut libc::c_void, buffer.len()) })
            as usize,
    )
}

pub struct Pipe {
    pub reader: RawFd,
    pub writer: RawFd,
}

pub fn pipe() -> io::Result<Pipe> {
    let mut pipefd: [libc::c_int; 2] = [0; 2];
    try_os!(unsafe { libc::pipe(pipefd.as_mut_ptr()) });
    Ok(Pipe {
        reader: pipefd[0],
        writer: pipefd[1],
    })
}

pub fn setsid() -> io::Result<libc::pid_t> {
    Ok(try_os!(unsafe { libc::setsid() }))
}

pub fn dup2(oldfd: libc::c_int, newfd: libc::c_int) -> io::Result<RawFd> {
    Ok(try_os!(unsafe { libc::dup2(oldfd, newfd) }))
}

pub fn fork() -> io::Result<Fork> {
    match try_os!(unsafe { libc::fork() }) {
        0 => Ok(Fork::Child),
        pid => Ok(Fork::Parent(pid)),
    }
}

pub enum Fork {
    Child,
    Parent(libc::pid_t),
}
