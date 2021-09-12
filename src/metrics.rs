use crate::{
    convert::time_to_string,
    models::{Data, DataPoint, DataPointType, Envelope, MetricData, Properties},
    tags::get_tags_for_metric,
    Exporter,
};
use opentelemetry::{
    metrics::{Descriptor, MetricsError, Result as MetricsResult},
    sdk::{
        export::metrics::{
            CheckpointSet, Count, ExportKind, ExportKindFor, ExportKindSelector,
            Exporter as MetricsExporter, LastValue, Max, Min, Points, Record, Sum,
        },
        metrics::aggregators::{
            ArrayAggregator, DdSketchAggregator, HistogramAggregator, LastValueAggregator,
            MinMaxSumCountAggregator, SumAggregator,
        },
    },
};
use std::convert::{TryFrom, TryInto};

#[cfg_attr(docsrs, doc(cfg(feature = "metrics")))]
impl<C> ExportKindFor for Exporter<C>
where
    C: std::fmt::Debug,
{
    fn export_kind_for(&self, descriptor: &Descriptor) -> ExportKind {
        ExportKindSelector::Stateless.export_kind_for(descriptor)
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "metrics")))]
impl<C> MetricsExporter for Exporter<C>
where
    C: std::fmt::Debug,
{
    fn export(&self, checkpoint_set: &mut dyn CheckpointSet) -> MetricsResult<()> {
        let mut envelopes = Vec::new();
        checkpoint_set.try_for_each(self, &mut |record| {
            let agg = record.aggregator().ok_or(MetricsError::NoDataCollected)?;
            let time = if let Some(last_value) = agg.as_any().downcast_ref::<LastValueAggregator>()
            {
                last_value.last_value()?.1
            } else {
                *record.end_time()
            };

            let tags = get_tags_for_metric(record);
            let data = Data::Metric(record.try_into()?);

            envelopes.push(Envelope {
                name: "Microsoft.ApplicationInsights.Metric".into(),
                time: time_to_string(time).into(),
                sample_rate: None,
                i_key: Some(self.instrumentation_key.clone().into()),
                tags: Some(tags).filter(|x| !x.is_empty()),
                data: Some(data),
            });

            Ok(())
        })?;

        crate::uploader::send_sync(&self.endpoint, envelopes)?;
        Ok(())
    }
}

impl TryFrom<&Record<'_>> for MetricData {
    type Error = MetricsError;

    fn try_from(record: &Record<'_>) -> Result<Self, Self::Error> {
        let agg = record.aggregator().ok_or(MetricsError::NoDataCollected)?;
        let desc = record.descriptor();
        let kind = desc.number_kind();

        let mut metrics = Vec::new();

        if let Some(array) = agg.as_any().downcast_ref::<ArrayAggregator>() {
            metrics = array
                .points()?
                .into_iter()
                .map(|n| DataPoint {
                    ns: None,
                    name: desc.name().into(),
                    value: n.to_f64(kind),
                    kind: Some(DataPointType::Measurement),
                })
                .collect();
        }

        if let Some(last_value) = agg.as_any().downcast_ref::<LastValueAggregator>() {
            let (value, _timestamp) = last_value.last_value()?;
            metrics.push(DataPoint {
                ns: None,
                name: desc.name().into(),
                kind: Some(DataPointType::Measurement),
                value: value.to_f64(kind),
            });
        }

        if let Some(mmsc) = agg.as_any().downcast_ref::<MinMaxSumCountAggregator>() {
            metrics.push(DataPoint {
                ns: None,
                name: desc.name().into(),
                kind: Some(DataPointType::Aggregation {
                    count: Some(mmsc.count()?.try_into().unwrap_or_default()),
                    min: Some(mmsc.min()?.to_f64(kind)),
                    max: Some(mmsc.max()?.to_f64(kind)),
                    std_dev: None,
                }),
                value: mmsc.sum()?.to_f64(kind),
            });
        }

        if let Some(dds) = agg.as_any().downcast_ref::<DdSketchAggregator>() {
            metrics.push(DataPoint {
                ns: None,
                name: desc.name().into(),
                kind: Some(DataPointType::Aggregation {
                    count: Some(dds.count()?.try_into().unwrap_or_default()),
                    min: Some(dds.min()?.to_f64(kind)),
                    max: Some(dds.max()?.to_f64(kind)),
                    std_dev: None,
                }),
                value: dds.sum()?.to_f64(kind),
            });
        }

        if let Some(sum) = agg.as_any().downcast_ref::<SumAggregator>() {
            metrics.push(DataPoint {
                ns: None,
                name: desc.name().into(),
                kind: Some(DataPointType::Aggregation {
                    count: None,
                    min: None,
                    max: None,
                    std_dev: None,
                }),
                value: sum.sum()?.to_f64(kind),
            });
        }

        if let Some(histogram) = agg.as_any().downcast_ref::<HistogramAggregator>() {
            metrics.push(DataPoint {
                ns: None,
                name: desc.name().into(),
                kind: Some(DataPointType::Aggregation {
                    count: Some(histogram.count()?.try_into().unwrap_or_default()),
                    min: None,
                    max: None,
                    std_dev: None,
                }),
                value: histogram.sum()?.to_f64(kind),
            });
        }

        let properties: Properties = record
            .resource()
            .iter()
            .chain(record.attributes().iter())
            .map(|(k, v)| (k.as_str().into(), v.into()))
            .collect();

        Ok(MetricData {
            ver: 2,
            metrics,
            properties: Some(properties).filter(|x| !x.is_empty()),
        })
    }
}
