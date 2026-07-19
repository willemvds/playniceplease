<p align="center">
  <img src="icon.svg" width="256" height="256" alt="icon" />
</p>

# Play Nice, Please!

## About
What does it do?
1) Listens for `SIGINT` (pressing ctrl+C) and `SIGTERM` (OS asking nicely to exit).
2) When a signal is received it sends `SIGTERM` to all sibling processes (other processes running inside the container).
3) After sending all the signals it checks up on the processes waiting for them to exit.
4) Collects exit codes for child processes from `SIGCHLD` allowing OS to free the PID and associated data.

What does it not do?
1) If a child process ignores the SIGTERM signal `playniceplease` will wait indefinitely. There are no timeout mechanisms because tools like `podman` will handle timeout and eventually kill the container. Having an interactive `zsh` session running is an example of this.

I am using `playniceplease` as the init process for all my development containers which typically consists of:
- Debian base image.
- Dev tools needed for project like compiler/package manager/dependencies.
- Opencode (for projects where it is relevant).

## Usage
- Download latest release from [Github Releases Page](https://github.com/willemvds/playniceplease/releases).
- Copy the binary to container and run it as the init process.

## Example `Containerfile`
```
COPY playniceplease /usr/local/bin/playniceplease

CMD ["/usr/local/bin/playniceplease"]
```

## Notices and Warnings
- This is intended as an init process for development containers. **DO NOT USE IN PRODUCTION**
- 100% of the code was produced by OpenCode + GLM 5.2. I have not reviewed the result in detail as you would for production quality software.


