use crate::Error;
use chrono::NaiveDateTime;
use rand::Rng;
use sntpc::{NtpContext, NtpTimestampGenerator, NtpUdpSocket, Result as SntpcR};
use std::{
    net::{SocketAddr, ToSocketAddrs, UdpSocket},
    time::Duration,
};
use thiserror::Error;

use super::TaskOutput;

#[derive(Error, Debug)]
pub enum NtpError {
    #[error("Sntpc Error: Unable to bind to any port")]
    NtpUnableToBindPort,

    #[error("Sntpc Error: Unable to set read timeout")]
    NtpUnableToSetReadTimeout,

    #[error("Invalid Unix timestamp: secs {0}, nanos {1}")]
    InvalidTimestampNanos(i64, u32),

    #[error("System time before unix epoch")]
    SystemTimeBeforeUnixEpoch,
}

#[derive(Copy, Clone, Default)]
struct StdTimestampGen {
    duration: Duration,
}

impl NtpTimestampGenerator for StdTimestampGen {
    fn init(&mut self) {
        self.duration = std::time::SystemTime::now()
            .duration_since(std::time::SystemTime::UNIX_EPOCH)
            .unwrap();
    }
    fn timestamp_sec(&self) -> u64 {
        self.duration.as_secs()
    }
    fn timestamp_subsec_micros(&self) -> u32 {
        self.duration.subsec_micros()
    }
}

#[derive(Debug)]
struct UdpSocketWrapper(UdpSocket);

impl NtpUdpSocket for UdpSocketWrapper {
    fn send_to<T: ToSocketAddrs>(&self, buf: &[u8], addr: T) -> SntpcR<usize> {
        match self.0.send_to(buf, addr) {
            Ok(usize) => Ok(usize),
            Err(_) => Err(sntpc::Error::Network),
        }
    }
    fn recv_from(&self, buf: &mut [u8]) -> SntpcR<(usize, SocketAddr)> {
        match self.0.recv_from(buf) {
            Ok((size, addr)) => Ok((size, addr)),
            Err(_) => Err(sntpc::Error::Network),
        }
    }
}

fn bind_to_random_port() -> std::io::Result<UdpSocket> {
    let mut rng = rand::thread_rng();
    let port_range = 49152..=65535;
    for _ in 0..1000 {
        let port = rng.gen_range(port_range.clone());
        match UdpSocket::bind(("0.0.0.0", port)) {
            Ok(socket) => {
                tracing::debug!("Bound to port {}", port);
                return Ok(socket);
            }
            Err(_) => continue,
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AddrNotAvailable,
        "Could not bind to any port in the range 49152-65535",
    ))
}

pub fn spawn_ntp_request(sender: tokio::sync::mpsc::Sender<Result<TaskOutput, Error>>) {
    tokio::spawn(async move {
        loop {
            tracing::debug!("Starting NTP request");
            let ntp_addrs = ntp_addrs();

            for addr in &ntp_addrs {
                let socket = match bind_to_random_port() {
                    Ok(socket) => socket,
                    Err(_e) => {
                        if let Err(e) = sender.send(Err(NtpError::NtpUnableToBindPort.into())).await
                        {
                            tracing::error!("Failed to send time to backend: {}", e);
                        }
                        return;
                    }
                };
                if let Err(_e) = socket.set_read_timeout(Some(Duration::from_secs(10))) {
                    if let Err(e) = sender
                        .send(Err(NtpError::NtpUnableToSetReadTimeout.into()))
                        .await
                    {
                        tracing::error!("Failed to send time to backend: {}", e);
                    }
                }
                let sock_wrapper = UdpSocketWrapper(socket);
                let ntp_context = NtpContext::new(StdTimestampGen::default());
                match sntpc::get_time(addr.1, sock_wrapper, ntp_context) {
                    Ok(time) => {
                        if let Err(e) = sender
                            .send(Ok(TaskOutput::Ntp(
                                ntp_total_microseconds(time),
                                addr.0.to_owned(),
                            )))
                            .await
                        {
                            tracing::error!("Failed to send time to backend: {}", e);
                        }
                        return;
                    }
                    Err(e) => {
                        tracing::error!("Failed to get time from server: {} - {:?}", addr.0, e);
                        // Continue trying the next address
                    }
                }
            }
            tracing::info!(
                "Failed to get NTP request. Trying again in 5 seconds in a different port."
            );
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });
}

fn ntp_addrs() -> Vec<(String, SocketAddr)> {
    let ntp_servers = vec![
        "pool.ntp.org:123",
        "time.google.com:123",
        "time.cloudflare.com:123",
        "time.windows.com:123",
        "time.apple.com:123",
        "time.facebook.com:123",
    ];

    let mut ntp_addrs: Vec<(String, std::net::SocketAddr)> = Vec::new();

    for server in &ntp_servers {
        match server.to_socket_addrs() {
            Ok(mut addrs) => {
                if let Some(addr) = addrs.next() {
                    ntp_addrs.push((server.to_string(), addr));
                }
            }
            Err(e) => {
                println!("Failed to resolve {}: {}", server, e);
            }
        }
    }
    ntp_addrs
}

fn ntp_total_microseconds(time: sntpc::NtpResult) -> u64 {
    let microseconds = time.sec_fraction() as u64 * 1_000_000u64 / u32::MAX as u64;
    let seconds = time.sec();
    let total_microseconds = seconds as u64 * 1_000_000u64 + microseconds;
    total_microseconds
}

// Helper function to get total microseconds from SystemTime
fn system_microseconds(time: std::time::SystemTime) -> Result<u64, NtpError> {
    match time.duration_since(std::time::UNIX_EPOCH) {
        Ok(since_the_epoch) => {
            let system_total_microseconds =
                since_the_epoch.as_secs() * 1_000_000 + since_the_epoch.subsec_micros() as u64;
            Ok(system_total_microseconds)
        }
        Err(_) => Err(NtpError::SystemTimeBeforeUnixEpoch),
    }
}

// Function to get current system time in total microseconds
pub fn system_now_microseconds() -> Result<u64, NtpError> {
    system_microseconds(std::time::SystemTime::now())
}

// Function to correct a time value (in microseconds) with an offset
pub fn correct_time_with_offset(time_microseconds: u64, offset: i64) -> std::time::SystemTime {
    let corrected_total_microseconds = (time_microseconds as i64 + offset) as u64;
    let corrected_secs = corrected_total_microseconds / 1_000_000;
    let corrected_subsec_micros = (corrected_total_microseconds % 1_000_000) as u32;
    let corrected_duration =
        std::time::Duration::new(corrected_secs, corrected_subsec_micros * 1_000);
    std::time::UNIX_EPOCH + corrected_duration
}

pub fn system_time_to_naive_utc(
    sys_time: std::time::SystemTime,
) -> Result<NaiveDateTime, NtpError> {
    // Duration since UNIX_EPOCH
    let duration = sys_time
        .duration_since(std::time::UNIX_EPOCH)
        .expect("Time went backwards");
    let secs = duration.as_secs() as i64;
    let nanos = duration.subsec_nanos();
    Ok(NaiveDateTime::from_timestamp_opt(secs, nanos)
        .ok_or(NtpError::InvalidTimestampNanos(secs, nanos))?)
}

#[cfg(test)]
mod tests {
    use chrono::{Datelike, NaiveDate, Timelike};

    use super::*;

    #[test]
    fn test_total_microseconds() {
        // Create a SystemTime from a known duration since the UNIX_EPOCH
        let duration = std::time::Duration::new(1_000_000, 0);
        let time = std::time::UNIX_EPOCH + duration;

        // Calculate the total microseconds
        let total_microsecs = system_microseconds(time).unwrap();

        // Assert that the total microseconds is correct
        assert_eq!(total_microsecs, 1_000_000_000_000);
    }

    #[test]
    fn test_get_system_time_microseconds() {
        // Get the current system time in total microseconds
        let total_microsecs = system_now_microseconds().unwrap();

        // Assert that the total microseconds is a positive number
        // (i.e., the system time is after the UNIX_EPOCH)
        assert!(total_microsecs > 0);
    }

    #[test]
    fn test_correct_time_with_offset() {
        // Create a SystemTime from a known duration since the UNIX_EPOCH
        let duration = std::time::Duration::new(1_000_000, 0);
        let time = std::time::UNIX_EPOCH + duration;

        // Calculate the total microseconds
        let total_microsecs = system_microseconds(time).unwrap();

        // Correct the time with a positive offset
        let offset = 1_000_000;
        let corrected_time = correct_time_with_offset(total_microsecs, offset);

        // Assert that the corrected time is correct
        let corrected_total_microsecs = system_microseconds(corrected_time).unwrap();
        assert_eq!(corrected_total_microsecs, total_microsecs + offset as u64);
    }

    #[test]
    fn test_system_time_to_naive_utc() {
        let sys_time = std::time::SystemTime::now();
        let naive_datetime = system_time_to_naive_utc(sys_time).unwrap();

        assert_eq!(
            naive_datetime.timestamp(),
            sys_time
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
        );

        // Test if date part is valid
        assert!(NaiveDate::from_ymd_opt(
            naive_datetime.year(),
            naive_datetime.month(),
            naive_datetime.day()
        )
        .is_some());

        // Test if time part is valid
        assert!(naive_datetime.hour() < 24);
        assert!(naive_datetime.minute() < 60);
        assert!(naive_datetime.second() < 60);
    }

    #[test]
    fn test_corrected_time() {
        // Simulate different offsets
        let offsets = [-1000000, 0, 1000000];

        for offset in offsets.iter() {
            // Get the current system time in total microseconds
            let system_total_microseconds = system_now_microseconds().unwrap();

            // Correct the system time with the offset and convert to NaiveDateTime
            let corrected_system_time =
                correct_time_with_offset(system_total_microseconds, *offset);
            let corrected_time = system_time_to_naive_utc(corrected_system_time).unwrap();

            // Calculate expected time
            let expected_system_time = correct_time_with_offset(system_total_microseconds, *offset);
            let expected_time = system_time_to_naive_utc(expected_system_time).unwrap();

            assert_eq!(corrected_time, expected_time);
        }
    }
}
