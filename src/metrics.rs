use crate::{
    convert::time_to_string,
    models::{Data, DataPoint, DataPointType, Envelope, MetricData, Properties},
    tags::get_tags_for_metric,
    Exporter,
};
use opentelemetry::{
    metrics::{MetricsError, Result as MetricsResult},
    sdk::{
        export::metrics::{
            aggregation::{
                stateless_temporality_selector, AggregationKind, Count, LastValue, Sum,
                Temporality, TemporalitySelector,
            },
            InstrumentationLibraryReader, MetricsExporter, Record,
        },
        metrics::{
            aggregators::{HistogramAggregator, LastValueAggregator, SumAggregator},
            sdk_api::Descriptor,
        },
        Resource,
    },
    Context,
};
use std::convert::{TryFrom, TryInto};

#[cfg_attr(docsrs, doc(cfg(feature = "metrics")))]
impl<C> TemporalitySelector for Exporter<C>
where
    C: std::fmt::Debug,
{
    fn temporality_for(&self, descriptor: &Descriptor, kind: &AggregationKind) -> Temporality {
        stateless_temporality_selector().temporality_for(descriptor, kind)
    }
}

#[cfg_attr(docsrs, doc(cfg(feature = "metrics")))]
impl<C> MetricsExporter for Exporter<C>
where
    C: std::fmt::Debug,
{
    fn export(
        &self,
        _cx: &Context,
        res: &Resource,
        reader: &dyn InstrumentationLibraryReader,
    ) -> MetricsResult<()> {
        let mut envelopes = Vec::new();
        reader.try_for_each(&mut |_library, reader| {
            reader.try_for_each(self, &mut |record| {
                let agg = record.aggregator().ok_or(MetricsError::NoDataCollected)?;
                let time =
                    if let Some(last_value) = agg.as_any().downcast_ref::<LastValueAggregator>() {
                        last_value.last_value()?.1
                    } else {
                        *record.end_time()
                    };

                let tags = get_tags_for_metric(record, res);
                let data = Data::Metric((record, res).try_into()?);

                envelopes.push(Envelope {
                    name: "Microsoft.ApplicationInsights.Metric".into(),
                    time: time_to_string(time).into(),
                    sample_rate: None,
                    i_key: Some(self.instrumentation_key.clone().into()),
                    tags: Some(tags).filter(|x| !x.is_empty()),
                    data: Some(data),
                });

                Ok(())
            })
        })?;

        crate::uploader::send_sync(&self.endpoint, envelopes)?;
        Ok(())
    }
}

impl TryFrom<(&Record<'_>, &Resource)> for MetricData {
    type Error = MetricsError;

    fn try_from((record, resource): (&Record<'_>, &Resource)) -> Result<Self, Self::Error> {
        let agg = record.aggregator().ok_or(MetricsError::NoDataCollected)?;
        let desc = record.descriptor();
        let kind = desc.number_kind();

        let mut metrics = Vec::new();

        if let Some(last_value) = agg.as_any().downcast_ref::<LastValueAggregator>() {
            let (value, _timestamp) = last_value.last_value()?;
            metrics.push(DataPoint {
                ns: None,
                name: desc.name().into(),
                kind: Some(DataPointType::Measurement),
                value: value.to_f64(kind),
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

        let properties: Properties = resource
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
