use crate::counter::{Atomic, Counter};
use crate::label::Label;
use crate::registry::Registry;
use std::io::Write;
use std::iter::{once, Once};

fn encode<W: Write, M: IterSamples>(
    writer: &mut W,
    registry: &Registry<M>,
) -> Result<(), std::io::Error> {
    for sample in registry.iter().map(IterSamples::iter_samples).flatten() {
        writer.write("metric_name".as_bytes())?;
        writer.write(" ".as_bytes())?;
        writer.write(sample.value.as_bytes())?;
    }

    writer.write("\n# EOF".as_bytes())?;

    Ok(())
}

struct Sample {
    suffix: Option<String>,
    labels: Option<Vec<Label>>,
    value: String,
}

trait IterSamples
where
    Self::IntoIter: Iterator<Item = Sample>,
{
    type IntoIter: Iterator;

    fn iter_samples(&self) -> Self::IntoIter;
}

impl<A> IterSamples for Counter<A>
where
    A: Atomic,
    A::Number: ToString,
{
    type IntoIter = Once<Sample>;

    fn iter_samples(&self) -> Once<Sample> {
        once(Sample {
            suffix: None,
            labels: None,
            value: self.get().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::counter::Counter;
    use pyo3::{
        prelude::*,
        types::{IntoPyDict, PyModule},
    };
    use std::sync::atomic::AtomicU64;

    #[test]
    fn register_and_iterate() {
        let mut registry = Registry::new();
        let counter = Counter::<AtomicU64>::new();
        registry.register(counter.clone());

        let mut encoded = Vec::new();

        encode(&mut encoded, &registry).unwrap();

        parse_with_python_client(String::from_utf8(encoded).unwrap());
    }

    fn parse_with_python_client(input: String) {
        Python::with_gil(|py| {
            let parser = PyModule::from_code(
                py,
                r#"
from prometheus_client.openmetrics.parser import text_string_to_metric_families

def parse(input):
    families = text_string_to_metric_families(input)
    list(families)
"#,
                "parser.py",
                "parser",
            )
            .map_err(|e| e.to_string())
            .unwrap();
            parser
                .call1("parse", (input,))
                .map_err(|e| e.to_string())
                .unwrap();
        })
    }
}
