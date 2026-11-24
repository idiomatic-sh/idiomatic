//! A small CPU record from the measured child, outside the retained cache.
use ryo_benchmark_contracts::domain::measurements::types::WorkerPhases;
use std::{
    fs, io,
    path::{Path, PathBuf},
    time::Duration,
};

const CLOCKS: &str = "worker-phases.tsv";

pub(crate) struct Clock {
    stamp: cpu_time::ProcessTime,
    samples: [Duration; 4],
    next: usize,
}

impl Clock {
    pub(crate) fn started() -> io::Result<Self> {
        Ok(Self {
            stamp: cpu_time::ProcessTime::try_now()?,
            samples: [Duration::ZERO; 4],
            next: 0,
        })
    }

    pub(crate) fn boundary(&mut self) -> io::Result<()> {
        let next = cpu_time::ProcessTime::try_now()?;
        let sample = self
            .samples
            .get_mut(self.next)
            .ok_or_else(|| io::Error::other("too many worker phase boundaries"))?;
        *sample = next.duration_since(self.stamp);
        self.next += 1;
        self.stamp = next;
        Ok(())
    }

    pub(crate) fn publish(self, path: &Path) -> io::Result<()> {
        if self.next != self.samples.len() {
            return Err(io::Error::other("incomplete worker phase record"));
        }
        write(path, self.samples)
    }
}

pub(crate) fn observation_file(request_state: &Path) -> Option<PathBuf> {
    request_state
        .parent()
        .map(|directory| directory.join(CLOCKS))
}

pub(crate) fn read(path: &Path) -> io::Result<Option<WorkerPhases>> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error),
    };
    let durations = text
        .split_ascii_whitespace()
        .map(|value| {
            value
                .parse::<u64>()
                .map(Duration::from_nanos)
                .map_err(io::Error::other)
        })
        .collect::<io::Result<Vec<_>>>()?;
    let [preparation, admission, execution, delivery] = durations.as_slice() else {
        return Err(io::Error::other("invalid worker phase record"));
    };
    Ok(Some(WorkerPhases {
        preparation: *preparation,
        admission: *admission,
        execution: *execution,
        delivery: *delivery,
    }))
}

fn write(
    path: &Path,
    [preparation, admission, execution, delivery]: [Duration; 4],
) -> io::Result<()> {
    fs::write(
        path,
        format!(
            "{}\t{}\t{}\t{}\n",
            preparation.as_nanos(),
            admission.as_nanos(),
            execution.as_nanos(),
            delivery.as_nanos()
        ),
    )
}
