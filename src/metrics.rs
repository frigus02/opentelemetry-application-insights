use crate::{
    convert::time_to_string,
    models::{Data, DataPoint, DataPointType, Envelope, MetricData, Properties},
    tags::get_tags_for_metric,
    Exporter,
};
use async_trait::async_trait;
use opentelemetry::{otel_warn, KeyValue};
use opentelemetry_http::HttpClient;
use opentelemetry_sdk::metrics::{
    data::{ExponentialHistogram, Gauge, Histogram, Metric, ResourceMetrics, Sum},
    exporter::PushMetricExporter,
    MetricResult, Temporality,
};
use std::{convert::TryInto, sync::Arc, time::SystemTime};

#[cfg_attr(docsrs, doc(cfg(feature = "metrics")))]
#[async_trait]
impl<C> PushMetricExporter for Exporter<C>
where
    C: HttpClient + 'static,
{
    async fn export(&self, metrics: &mut ResourceMetrics) -> MetricResult<()> {
        let client = Arc::clone(&self.client);
        let endpoint = Arc::clone(&self.endpoint);

        let mut envelopes = Vec::new();
        for scope_metrics in metrics.scope_metrics.iter() {
            for metric in scope_metrics.metrics.iter() {
                let data_points = map_metric(metric);
                for data in data_points {
                    let tags =
                        get_tags_for_metric(&metrics.resource, &scope_metrics.scope, &data.attrs);
                    let properties: Properties = metrics
                        .resource
                        .iter()
                        .chain(
                            scope_metrics
                                .scope
                                .attributes()
                                .map(|kv| (&kv.key, &kv.value)),
                        )
                        .chain(data.attrs.iter().map(|kv| (&kv.key, &kv.value)))
                        .map(|(k, v)| (k.as_str().into(), v.into()))
                        .collect();
                    envelopes.push(Envelope {
                        name: "Microsoft.ApplicationInsights.Metric",
                        time: time_to_string(data.time).into(),
                        sample_rate: None,
                        i_key: Some(self.instrumentation_key.clone().into()),
                        tags: Some(tags).filter(|x| !x.is_empty()),
                        data: Some(Data::Metric(MetricData {
                            ver: 2,
                            metrics: vec![data.data],
                            properties: Some(properties).filter(|x| !x.is_empty()),
                        })),
                    });
                }
            }
        }

        crate::uploader::send(client.as_ref(), endpoint.as_ref(), envelopes).await?;
        Ok(())
    }

    async fn force_flush(&self) -> MetricResult<()> {
        Ok(())
    }

    fn shutdown(&self) -> MetricResult<()> {
        Ok(())
    }

    fn temporality(&self) -> Temporality {
        // Application Insights only supports Delta temporality as defined in the spec:
        //
        // > Choose Delta aggregation temporality for Counter, Asynchronous Counter and Histogram
        // > instrument kinds, choose Cumulative aggregation for UpDownCounter and Asynchronous
        // > UpDownCounter instrument kinds.
        //
        // See:
        // - https://github.com/open-telemetry/opentelemetry-specification/blob/58bfe48eabe887545198d66c43f44071b822373f/specification/metrics/sdk_exporters/otlp.md?plain=1#L46-L47
        // - https://github.com/frigus02/opentelemetry-application-insights/issues/74#issuecomment-2108488385
        Temporality::Delta
    }
}

struct EnvelopeData {
    time: SystemTime,
    data: DataPoint,
    attrs: Vec<KeyValue>,
}

trait ToF64Lossy {
    fn to_f64_lossy(&self) -> f64;
}

impl ToF64Lossy for i64 {
    fn to_f64_lossy(&self) -> f64 {
        *self as f64
    }
}

impl ToF64Lossy for u64 {
    fn to_f64_lossy(&self) -> f64 {
        *self as f64
    }
}

impl ToF64Lossy for f64 {
    fn to_f64_lossy(&self) -> f64 {
        *self
    }
}

fn map_metric(metric: &Metric) -> Vec<EnvelopeData> {
    let data = metric.data.as_any();
    if let Some(gauge) = data.downcast_ref::<Gauge<u64>>() {
        map_gauge(metric, gauge)
    } else if let Some(gauge) = data.downcast_ref::<Gauge<i64>>() {
        map_gauge(metric, gauge)
    } else if let Some(gauge) = data.downcast_ref::<Gauge<f64>>() {
        map_gauge(metric, gauge)
    } else if let Some(histogram) = data.downcast_ref::<Histogram<i64>>() {
        map_histogram(metric, histogram)
    } else if let Some(histogram) = data.downcast_ref::<Histogram<u64>>() {
        map_histogram(metric, histogram)
    } else if let Some(histogram) = data.downcast_ref::<Histogram<f64>>() {
        map_histogram(metric, histogram)
    } else if let Some(exp_histogram) = data.downcast_ref::<ExponentialHistogram<i64>>() {
        map_exponential_histogram(metric, exp_histogram)
    } else if let Some(exp_histogram) = data.downcast_ref::<ExponentialHistogram<u64>>() {
        map_exponential_histogram(metric, exp_histogram)
    } else if let Some(exp_histogram) = data.downcast_ref::<ExponentialHistogram<f64>>() {
        map_exponential_histogram(metric, exp_histogram)
    } else if let Some(sum) = data.downcast_ref::<Sum<u64>>() {
        map_sum(metric, sum)
    } else if let Some(sum) = data.downcast_ref::<Sum<i64>>() {
        map_sum(metric, sum)
    } else if let Some(sum) = data.downcast_ref::<Sum<f64>>() {
        map_sum(metric, sum)
    } else {
        otel_warn!(name: "ApplicationInsights.ExportMetrics.UnknownAggregator");
        Vec::new()
    }
}

fn map_gauge<T: ToF64Lossy>(metric: &Metric, gauge: &Gauge<T>) -> Vec<EnvelopeData> {
    gauge
        .data_points
        .iter()
        .map(|data_point| {
            let time = data_point
                .time
                .or(data_point.start_time)
                .unwrap_or_else(SystemTime::now);
            let data = DataPoint {
                ns: None,
                name: metric.name.clone().into(),
                kind: Some(DataPointType::Measurement),
                value: data_point.value.to_f64_lossy(),
            };
            let attrs = data_point.attributes.to_owned();
            EnvelopeData { time, data, attrs }
        })
        .collect()
}

fn map_histogram<T: ToF64Lossy>(metric: &Metric, histogram: &Histogram<T>) -> Vec<EnvelopeData> {
    histogram
        .data_points
        .iter()
        .map(|data_point| {
            let time = data_point.time;
            let data = DataPoint {
                ns: None,
                name: metric.name.clone().into(),
                kind: Some(DataPointType::Aggregation {
                    count: Some(data_point.count.try_into().unwrap_or_default()),
                    min: data_point.min.as_ref().map(ToF64Lossy::to_f64_lossy),
                    max: data_point.max.as_ref().map(ToF64Lossy::to_f64_lossy),
                    std_dev: None,
                }),
                value: data_point.sum.to_f64_lossy(),
            };
            let attrs = data_point.attributes.to_owned();
            EnvelopeData { time, data, attrs }
        })
        .collect()
}

fn map_exponential_histogram<T: ToF64Lossy>(
    metric: &Metric,
    exp_histogram: &ExponentialHistogram<T>,
) -> Vec<EnvelopeData> {
    exp_histogram
        .data_points
        .iter()
        .map(|data_point| {
            let time = data_point.time;
            let data = DataPoint {
                ns: None,
                name: metric.name.clone().into(),
                kind: Some(DataPointType::Aggregation {
                    count: Some(data_point.count.try_into().unwrap_or_default()),
                    min: data_point.min.as_ref().map(ToF64Lossy::to_f64_lossy),
                    max: data_point.max.as_ref().map(ToF64Lossy::to_f64_lossy),
                    std_dev: None,
                }),
                value: data_point.sum.to_f64_lossy(),
            };
            let attrs = data_point.attributes.to_owned();
            EnvelopeData { time, data, attrs }
        })
        .collect()
}

fn map_sum<T: ToF64Lossy>(metric: &Metric, sum: &Sum<T>) -> Vec<EnvelopeData> {
    sum.data_points
        .iter()
        .map(|data_point| {
            let time = data_point
                .time
                .or(data_point.start_time)
                .unwrap_or_else(SystemTime::now);
            let data = DataPoint {
                ns: None,
                name: metric.name.clone().into(),
                kind: Some(DataPointType::Aggregation {
                    count: None,
                    min: None,
                    max: None,
                    std_dev: None,
                }),
                value: data_point.value.to_f64_lossy(),
            };
            let attrs = data_point.attributes.to_owned();
            EnvelopeData { time, data, attrs }
        })
        .collect()
}
