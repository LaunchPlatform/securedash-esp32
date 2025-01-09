use std::cmp::{max, min};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::time::{Duration, SystemTime};

#[derive(Debug)]
pub struct StorageBenchmark {
    pub path: String,
    pub file_size: usize,
    pub chunk_size: usize,
    pub loop_times: usize,
}

impl StorageBenchmark {
    pub fn run_once(&self) -> anyhow::Result<Duration> {
        log::info!("Running write test with config={self:#?}");
        let buf: Vec<u8> = vec![1; self.chunk_size];
        let mut remaining_size = self.file_size;

        let start = SystemTime::now();
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .open(&self.path)?;
        while remaining_size > 0 {
            let chunk_size = min(self.chunk_size, remaining_size);
            let _written = file.write(&buf[..chunk_size])?;
            // Notice: somehow there's a bug or what making written size returning 0, which is
            //         obviously wrong. Before we fix that, we just calculate the remaining_size
            //         based on the size we have written
            remaining_size -= chunk_size;
        }
        file.sync_all()?;
        let end = SystemTime::now();
        let duration = end.duration_since(start)?;
        log::info!(
            "Finish write test with total time {} sec",
            duration.as_secs()
        );
        log::info!(
            "throughput={} MB/s",
            ((self.file_size as f64) / (1024.0 * 1024.0)) / duration.as_secs_f64()
        );
        Ok(duration)
    }

    pub fn run(&self) -> anyhow::Result<()> {
        let mut durations = Vec::<Duration>::with_capacity(self.loop_times);
        for idx in (0..self.loop_times) {
            log::info!(
                "Running write test for {} out of {} time",
                idx + 1,
                self.loop_times
            );
            let duration = self.run_once()?;
            durations.push(duration);
        }

        let total: Duration = durations.iter().sum();
        log::info!(
            "Finish {} write test with total time {} sec",
            self.loop_times,
            total.as_secs()
        );
        log::info!(
            "avg throughput={} MB/s",
            ((self.file_size as f64) * (self.loop_times as f64) / (1024.0 * 1024.0))
                / total.as_secs_f64()
        );

        Ok(())
    }
}
