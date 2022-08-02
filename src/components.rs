use crate::{
    parser::{self, Component},
    property::{
        DateDateTimeOrPeriod, DateOrDateTime, EndCondition, IcalDateTime, Offseter, Property,
        RecurRule, ToNaive, ToNaivePeriod,
    },
};
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::convert::{TryFrom, TryInto};

use anyhow::{bail, ensure, format_err, Context, Error};
use chrono::{DateTime, Duration, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Utc};
use itertools::Itertools;

#[derive(Debug, Clone)]
pub struct VCalendar {
    pub prodid: String,
    pub version: String,

    // TODO: Add other components.
    pub events: BTreeMap<String, EventCollection>,
    pub timezones: Vec<VTimeZone>,

    pub properties: Vec<Property>,
}

impl VCalendar {
    pub fn get_time(&self, date: &IcalDateTime) -> Result<DateTime<FixedOffset>, Error> {
        match *date {
            IcalDateTime::Local(_) => bail!("Local time"),
            IcalDateTime::Utc(d) => Ok(d.into()),
            IcalDateTime::TZ { date, ref tzid } => {
                let tz = if let Some(tz) = self.timezones.iter().find(|tz| &tz.id == tzid) {
                    tz.clone()
                } else {
                    bail!("Referenced timezone {} not in calendar", tzid);
                };

                Ok(tz.to_instance(date))
            }
        }
    }
}

impl TryFrom<parser::Component> for VCalendar {
    type Error = Error;

    fn try_from(component: parser::Component) -> Result<Self, Self::Error> {
        ensure!(component.name.to_ascii_uppercase() == "VCALENDAR");

        let mut vevents = Vec::new();
        let mut timezones = Vec::new();
        for component in component.sub_components {
            match &component.name.to_ascii_uppercase() as &str {
                "VEVENT" => {
                    // We parse VEvents after everything else, so that it can
                    // access the timezone info.
                    vevents.push(component);
                }
                "VTIMEZONE" => {
                    timezones.push(component.try_into().with_context(|| "parsing VTIMEZONE")?)
                }
                _ => {} // TODO: Handle other components
            }
        }

        let mut prodid = None;
        let mut version = None;

        let mut properties = Vec::new();
        for prop in component.properties {
            let parsed: Property = prop.try_into()?;

            match parsed {
                Property::ProductIdentifier(value) => prodid = Some(value.value),
                Property::Version(value) => version = Some(value.value),
                p => properties.push(p),
            }
        }

        let mut vcalendar = VCalendar {
            prodid: prodid.ok_or_else(|| format_err!("Missing PRODID field in offset rule"))?,
            version: version.ok_or_else(|| format_err!("Missing VERSION field in offset rule"))?,
            events: BTreeMap::new(),
            timezones,
            properties,
        };

        let mut events: BTreeMap<String, Vec<VEvent>> = BTreeMap::new();
        for component in vevents {
            let event = VEvent::try_from_component(component, &vcalendar)
                .with_context(|| "parsing VEVENT")?;
            events.entry(event.uid.clone()).or_default().push(event);
        }

        vcalendar.events = events
            .into_iter()
            .map(|(uid, events)| -> Result<_, Error> { Ok((uid, EventCollection::new(events)?)) })
            .collect::<Result<_, _>>()?;

        Ok(vcalendar)
    }
}

//// Purpose: Provide a grouping of component properties that describe an event.
///
/// Description:  A "VEVENT" calendar component is a grouping of component
/// properties, possibly including "VALARM" calendar components, that represents
/// a scheduled amount of time on a calendar.  For example, it can be an
/// activity; such as a one-hour long, department meeting from 8:00 AM to 9:00
/// AM, tomorrow. Generally, an event will take up time on an individual
/// calendar. Hence, the event will appear as an opaque interval in a search for
/// busy time.  Alternately, the event can have its Time Transparency set to
/// "TRANSPARENT" in order to prevent blocking of the event in searches for busy
/// time.
///
/// The "VEVENT" is also the calendar component used to specify an anniversary
/// or daily reminder within a calendar.  These events have a DATE value type
/// for the "DTSTART" property instead of the default value type of DATE-TIME.
/// If such a "VEVENT" has a "DTEND" property, it MUST be specified as a DATE
/// value also.  The anniversary type of "VEVENT" can span more than one date
/// (i.e., "DTEND" property value is set to a calendar date after the "DTSTART"
/// property value).  If such a "VEVENT" has a "DURATION" property, it MUST be
/// specified as a "dur-day" or "dur-week" value.
///
/// The "DTSTART" property for a "VEVENT" specifies the inclusive start of the
/// event.  For recurring events, it also specifies the very first instance in
/// the recurrence set.  The "DTEND" property for a "VEVENT" calendar component
/// specifies the non-inclusive end of the event.  For cases where a "VEVENT"
/// calendar component specifies a "DTSTART" property with a DATE value type but
/// no "DTEND" nor "DURATION" property, the event's duration is taken to be one
/// day.  For cases where a "VEVENT" calendar component specifies a "DTSTART"
/// property with a DATE-TIME value type but no "DTEND" property, the event ends
/// on the same calendar date and time of day specified by the "DTSTART"
/// property.
///
/// The "VEVENT" calendar component cannot be nested within another calendar
/// component.  However, "VEVENT" calendar components can be related to each
/// other or to a "VTODO" or to a "VJOURNAL" calendar component with the
/// "RELATED-TO" property.
#[derive(Debug, Clone)]
pub struct VEvent {
    pub uid: String,
    pub dtstamp: DateTime<Utc>,
    pub summary: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub sequence: Option<u32>,
    pub recur: Option<RecurRule>,
    pub timings: Option<Timings>,

    is_recurrence_instance: bool,

    pub properties: Vec<Property>,
}

impl VEvent {
    /// Whether the event is for a full day.
    pub fn is_full_day_event(&self) -> bool {
        matches!(
            self.timings,
            Some(Timings::Date(_)) | Some(Timings::PerioidDate(_))
        )
    }

    /// Whether this is a "floating" event, which are not bound to any
    /// particular timezone.
    ///
    ///
    /// Floating events are used to represent the same hour, minute, and second
    /// value regardless of which time zone is currently being observed.  For
    /// example, an event can be defined that indicates that an individual will
    /// be busy from 11:00 AM to 1:00 PM every day, no matter which time zone
    /// the person is in.
    pub fn is_floating_event(&self) -> bool {
        matches!(
            self.timings,
            Some(Timings::Local(_)) | Some(Timings::PerioidLocal(_))
        )
    }

    /// Get an iterator over all instances of the event, with timezone
    /// information.
    ///
    /// This will fail if it is a floating event or if there is a referenced
    /// timezone that can't be found in the given `VCalendar`.
    ///
    /// Note: This may be an infinite iterator if the event recurs forever.
    pub fn recur_iter<'a>(
        &'a self,
        calendar: &'a VCalendar,
    ) -> Result<impl Iterator<Item = DateTime<FixedOffset>> + 'a, Error> {
        let recur = if let Some(recur) = &self.recur {
            recur
        } else {
            return match &self.timings {
                Some(Timings::Utc(inner)) => Ok(Box::new(std::iter::once(
                    FixedOffset::east(0).from_utc_datetime(&inner.start.naive_utc()),
                ))
                    as Box<dyn Iterator<Item = DateTime<FixedOffset>>>),
                Some(Timings::Tz { tzid, inner }) => {
                    let tz = if let Some(tz) = calendar.timezones.iter().find(|tz| &tz.id == tzid) {
                        tz.clone()
                    } else {
                        bail!("Referenced timezone {} not in calendar", tzid);
                    };

                    Ok(Box::new(std::iter::once(tz.to_instance(inner.start)))
                        as Box<dyn Iterator<Item = DateTime<FixedOffset>>>)
                }
                Some(Timings::PerioidUtc(inner)) => Ok(Box::new(std::iter::once(
                    FixedOffset::east(0).from_utc_datetime(&inner.start.start.naive_utc()),
                ))
                    as Box<dyn Iterator<Item = DateTime<FixedOffset>>>),
                Some(Timings::PerioidTz { tzid, inner }) => {
                    let tz = if let Some(tz) = calendar.timezones.iter().find(|tz| &tz.id == tzid) {
                        tz.clone()
                    } else {
                        bail!("Referenced timezone {} not in calendar", tzid);
                    };

                    Ok(Box::new(std::iter::once(tz.to_instance(inner.start.start)))
                        as Box<dyn Iterator<Item = DateTime<FixedOffset>>>)
                }
                _ => bail!("Not a datetime event"),
            };
        };

        match &self.timings {
            Some(Timings::Utc(inner)) => Ok(Box::new(
                recur
                    .from_date_with_extras(
                        inner.start,
                        inner.rdates.iter().cloned(),
                        &inner.exdates,
                        FixedOffset::east(0),
                    )
                    .map(|d| d.into()),
            )
                as Box<dyn Iterator<Item = DateTime<FixedOffset>>>),
            Some(Timings::Tz { tzid, inner }) => {
                let tz = if let Some(tz) = calendar.timezones.iter().find(|tz| &tz.id == tzid) {
                    tz.clone()
                } else {
                    bail!("Referenced timezone {} not in calendar", tzid);
                };

                Ok(Box::new(recur.from_naive_date_with_extras(
                    inner.start,
                    inner.rdates.iter().cloned(),
                    &inner.exdates,
                    tz,
                ))
                    as Box<dyn Iterator<Item = DateTime<FixedOffset>>>)
            }
            Some(Timings::PerioidUtc(inner)) => Ok(Box::new(
                recur
                    .from_date_with_extras(
                        inner.start.start,
                        inner.rdates.iter().map(|d| d.start),
                        &inner.exdates,
                        FixedOffset::east(0),
                    )
                    .map(|d| d.into()),
            )
                as Box<dyn Iterator<Item = DateTime<FixedOffset>>>),
            Some(Timings::PerioidTz { tzid, inner }) => {
                let tz = if let Some(tz) = calendar.timezones.iter().find(|tz| &tz.id == tzid) {
                    tz.clone()
                } else {
                    bail!("Referenced timezone {} not in calendar", tzid);
                };

                Ok(Box::new(recur.from_naive_date_with_extras(
                    inner.start.start,
                    inner.rdates.iter().map(|d| d.start),
                    &inner.exdates,
                    tz,
                ))
                    as Box<dyn Iterator<Item = DateTime<FixedOffset>>>)
            }
            _ => bail!("Not a datetime event"),
        }
    }

    pub fn recur_period_iter<'a>(
        &'a self,
        calendar: &'a VCalendar,
    ) -> Result<impl Iterator<Item = ToNaivePeriod<DateTime<FixedOffset>>> + 'a, Error> {
        let recur = if let Some(recur) = &self.recur {
            recur
        } else {
            return match &self.timings {
                Some(Timings::PerioidUtc(inner)) => Ok(Box::new(std::iter::once(ToNaivePeriod {
                    duration: inner.start.duration,
                    start: FixedOffset::east(0).from_utc_datetime(&inner.start.start.naive_utc()),
                }))
                    as Box<dyn Iterator<Item = _>>),
                Some(Timings::PerioidTz { tzid, inner }) => {
                    let tz = if let Some(tz) = calendar.timezones.iter().find(|tz| &tz.id == tzid) {
                        tz.clone()
                    } else {
                        bail!("Referenced timezone {} not in calendar", tzid);
                    };

                    Ok(Box::new(std::iter::once(ToNaivePeriod {
                        duration: inner.start.duration,
                        start: tz.to_instance(inner.start.start),
                    })) as Box<dyn Iterator<Item = _>>)
                }
                _ => bail!("Not a datetime event"),
            };
        };

        match &self.timings {
            Some(Timings::PerioidUtc(inner)) => Ok(Box::new(
                recur
                    .from_date_with_extras(
                        inner.start,
                        inner.rdates.iter().cloned(),
                        &inner.exdates,
                        FixedOffset::east(0),
                    )
                    .map(|d| ToNaivePeriod {
                        duration: d.duration,
                        start: d.start.into(),
                    }),
            ) as Box<dyn Iterator<Item = _>>),
            Some(Timings::PerioidTz { tzid, inner }) => {
                let tz = if let Some(tz) = calendar.timezones.iter().find(|tz| &tz.id == tzid) {
                    tz.clone()
                } else {
                    bail!("Referenced timezone {} not in calendar", tzid);
                };

                Ok(
                    Box::new(recur.from_naive_date_with_extras::<ToNaivePeriod<DateTime<FixedOffset>>, NaiveDateTime, _, _>(
                        inner.start.to_naive(),
                        inner.rdates.iter().map(ToNaive::to_naive),
                        &inner.exdates,
                        tz,
                    )) as Box<dyn Iterator<Item = _>>,
                )
            }
            _ => bail!("Not a datetime event"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TimingsInner<T, E = T> {
    start: T,
    exdates: Vec<E>,
    rdates: Vec<T>,
    recur_id: Option<E>,
}

#[derive(Debug, Clone)]
pub enum Timings {
    Date(TimingsInner<NaiveDate>),
    Local(TimingsInner<NaiveDateTime>),
    Utc(TimingsInner<DateTime<Utc>>),
    Tz {
        tzid: String,
        inner: TimingsInner<NaiveDateTime>,
    },

    PerioidDate(TimingsInner<ToNaivePeriod<NaiveDate>, NaiveDate>),
    PerioidLocal(TimingsInner<ToNaivePeriod<NaiveDateTime>, NaiveDateTime>),
    PerioidUtc(TimingsInner<ToNaivePeriod<DateTime<Utc>>, DateTime<Utc>>),
    PerioidTz {
        tzid: String,
        inner: TimingsInner<ToNaivePeriod<NaiveDateTime>, NaiveDateTime>,
    },
}

impl VEvent {
    /// Try to convert the component into a [`VEvent`], in the context of the
    /// given calendar.
    ///
    /// We pass in the calendar mainly so they have access to timezone information.
    fn try_from_component(
        component: parser::Component,
        calendar: &VCalendar,
    ) -> Result<Self, Error> {
        ensure!(component.name.to_ascii_uppercase() == "VEVENT");

        // TODO: Handle sub compontents

        let mut uid = None;
        let mut dtstamp = None;
        let mut recur = None;
        let mut dtstart = None;
        let mut rdates = Vec::new();
        let mut exdates = Vec::new();
        let mut duration = None;
        let mut dtend = None;
        let mut recur_id = None;
        let mut summary = None;
        let mut description = None;
        let mut location = None;
        let mut sequence = None;

        let mut properties = Vec::new();
        for prop in component.properties {
            let parsed: Property = prop.try_into()?;

            match parsed {
                Property::RecurrenceRule(value) => recur = Some(value.value),
                Property::UID(value) => uid = Some(value.value),
                Property::DateTimeStamp(value) => dtstamp = Some(value.value),
                Property::Start(value) => dtstart = Some(value.value),
                Property::RecurrenceDateTimes(value) => rdates.push(value.value),
                Property::ExceptionDateTimes(value) => exdates.push(value.value),
                Property::Duration(value) => duration = Some(value.value),
                Property::End(value) => dtend = Some(value.value),
                Property::RecurrenceID(value) => recur_id = Some(value.value),
                Property::Summary(value) => summary = Some(value.value),
                Property::Description(value) => description = Some(value.value),
                Property::Location(value) => location = Some(value.value),
                Property::SequenceNumber(value) => sequence = Some(value.value),
                p => properties.push(p),
            }
        }

        let uid = uid.ok_or_else(|| format_err!("Missing UID field in offset rule"))?;

        if duration.is_some() && dtend.is_some() {
            bail!("VEVENT has both DURATION and DTEND");
        }

        if let Some(dtend) = dtend {
            if let Some(dtstart) = &dtstart {
                duration = Some(match (dtstart.clone(), dtend) {
                    (DateOrDateTime::Date(start), DateOrDateTime::Date(end)) => end - start,
                    (DateOrDateTime::DateTime(start), DateOrDateTime::DateTime(end)) => end
                        .sub(&start, Some(calendar))
                        .with_context(|| format!("calculating duration for {}", uid))?,
                    _ => bail!("VEVENT has different types for DTSTART and DTEND"),
                });
            } else {
                bail!("VEVENT has a DTEND without DTSTART")
            }
        };

        let is_recurrence_instance = recur_id.is_some();

        // To make the code a bit simpler we convert the recurrence ID into an
        // offset from the start time.
        let mut recur_offset = None;

        if let Some(recur_id) = recur_id {
            if let Some(dtstart) = &dtstart {
                recur_offset = Some(match (dtstart.clone(), recur_id) {
                    (DateOrDateTime::Date(start), DateOrDateTime::Date(recur)) => recur - start,
                    (DateOrDateTime::DateTime(start), DateOrDateTime::DateTime(recur)) => recur
                        .sub(&start, Some(calendar))
                        .with_context(|| format!("calculating recur ID offset for {}", uid))?,
                    _ => bail!("VEVENT has different types for DTSTART and RECURRENCE-ID"),
                });
            } else {
                bail!("VEVENT has a RECURRENCE-ID without DTSTART")
            }
        }

        let timings = if let Some(duration) = duration {
            match dtstart {
                Some(DateOrDateTime::Date(start)) => Some(Timings::PerioidDate(TimingsInner {
                    start: ToNaivePeriod { start, duration },
                    exdates: try_to_dates(exdates)?,
                    rdates: try_from_period_to_periods(duration, rdates)?,
                    recur_id: recur_offset.map(|offset| start + offset),
                })),
                Some(DateOrDateTime::DateTime(cal)) => match cal {
                    IcalDateTime::Local(start) => Some(Timings::PerioidLocal(TimingsInner {
                        start: ToNaivePeriod { start, duration },
                        exdates: try_to_dates(exdates)?,
                        rdates: try_from_period_to_periods(duration, rdates)?,
                        recur_id: recur_offset.map(|offset| start + offset),
                    })),
                    IcalDateTime::Utc(start) => Some(Timings::PerioidUtc(TimingsInner {
                        start: ToNaivePeriod { start, duration },
                        exdates: try_to_dates(exdates)?,
                        rdates: try_from_period_to_periods(duration, rdates)?,
                        recur_id: recur_offset.map(|offset| start + offset),
                    })),
                    IcalDateTime::TZ { date, tzid } => {
                        let inner = TimingsInner {
                            start: ToNaivePeriod {
                                start: date,
                                duration,
                            },
                            exdates: try_tz_to_dates(&tzid, exdates)?,
                            rdates: try_tz_from_period_to_periods(duration, &tzid, rdates)?,
                            recur_id: recur_offset.map(|offset| date + offset),
                        };
                        Some(Timings::PerioidTz { tzid, inner })
                    }
                },
                None => None,
            }
        } else {
            match dtstart {
                Some(DateOrDateTime::Date(start)) => Some(Timings::Date(TimingsInner {
                    start,
                    exdates: try_to_dates(exdates)?,
                    rdates: try_from_period_to_dates(rdates)?,
                    recur_id: recur_offset.map(|offset| start + offset),
                })),
                Some(DateOrDateTime::DateTime(cal)) => match cal {
                    IcalDateTime::Local(start) => Some(Timings::Local(TimingsInner {
                        start,
                        exdates: try_to_dates(exdates)?,
                        rdates: try_from_period_to_dates(rdates)?,
                        recur_id: recur_offset.map(|offset| start + offset),
                    })),
                    IcalDateTime::Utc(start) => Some(Timings::Utc(TimingsInner {
                        start,
                        exdates: try_to_dates(exdates)?,
                        rdates: try_from_period_to_dates(rdates)?,
                        recur_id: recur_offset.map(|offset| start + offset),
                    })),
                    IcalDateTime::TZ { date, tzid } => {
                        let inner = TimingsInner {
                            start: date,
                            exdates: try_tz_to_dates(&tzid, exdates)?,
                            rdates: try_period_tz_to_dates(&tzid, rdates)?,
                            recur_id: recur_offset.map(|offset| date + offset),
                        };
                        Some(Timings::Tz { tzid, inner })
                    }
                },
                None => None,
            }
        };

        Ok(VEvent {
            uid,
            dtstamp: dtstamp.ok_or_else(|| format_err!("Missing DTSTAMP field in offset rule"))?,
            recur,
            summary,
            description,
            location,
            sequence,
            timings,
            properties,
            is_recurrence_instance,
        })
    }
}

fn try_tz_to_dates(
    expected_tzid: &str,
    vec: Vec<DateOrDateTime>,
) -> Result<Vec<NaiveDateTime>, Error> {
    let mut dates = Vec::with_capacity(vec.len());

    for d in vec {
        match d {
            DateOrDateTime::DateTime(IcalDateTime::TZ { tzid, date }) => {
                if tzid != expected_tzid {
                    bail!("TZ mismatch")
                }
                dates.push(date)
            }
            _ => bail!("DateTime mismatch"),
        }
    }

    Ok(dates)
}

fn try_to_dates<D: TryFrom<DateOrDateTime>>(vec: Vec<DateOrDateTime>) -> Result<Vec<D>, D::Error> {
    let mut dates = Vec::with_capacity(vec.len());

    for d in vec {
        dates.push(d.try_into()?);
    }

    Ok(dates)
}

fn try_period_tz_to_dates(
    expected_tzid: &str,
    vec: Vec<DateDateTimeOrPeriod>,
) -> Result<Vec<NaiveDateTime>, Error> {
    let mut dates = Vec::with_capacity(vec.len());

    for d in vec {
        match d {
            DateDateTimeOrPeriod::DateTime(IcalDateTime::TZ { tzid, date }) => {
                if tzid != expected_tzid {
                    bail!("TZ mismatch")
                }
                dates.push(date)
            }
            _ => bail!("DateTime mismatch"),
        }
    }

    Ok(dates)
}

fn try_from_period_to_dates<D: TryFrom<DateDateTimeOrPeriod>>(
    vec: Vec<DateDateTimeOrPeriod>,
) -> Result<Vec<D>, D::Error> {
    let mut dates = Vec::with_capacity(vec.len());

    for d in vec {
        dates.push(d.try_into()?);
    }

    Ok(dates)
}

fn try_from_period_to_periods<D: TryFrom<DateDateTimeOrPeriod>>(
    duration: Duration,
    vec: Vec<DateDateTimeOrPeriod>,
) -> Result<Vec<ToNaivePeriod<D>>, D::Error>
where
    D: ToNaive,
{
    let mut dates = Vec::with_capacity(vec.len());

    for d in vec {
        match d {
            DateDateTimeOrPeriod::Period(period) => dates.push(ToNaivePeriod {
                start: DateDateTimeOrPeriod::DateTime(period.start).try_into()?,
                duration: period.duration,
            }),
            _ => dates.push(ToNaivePeriod {
                start: d.try_into()?,
                duration,
            }),
        }
    }

    Ok(dates)
}

fn try_tz_from_period_to_periods(
    duration: Duration,
    expected_tzid: &str,
    vec: Vec<DateDateTimeOrPeriod>,
) -> Result<Vec<ToNaivePeriod<NaiveDateTime>>, Error> {
    let mut dates = Vec::with_capacity(vec.len());

    for d in vec {
        match d {
            DateDateTimeOrPeriod::Period(period) => {
                match period.start {
                    IcalDateTime::TZ { date, tzid } => {
                        if tzid != expected_tzid {
                            bail!("TZ mismatch")
                        }
                        dates.push(ToNaivePeriod {
                            start: date,
                            duration: period.duration,
                        })
                    }
                    _ => bail!("DateTime mismatch"),
                };
            }

            DateDateTimeOrPeriod::DateTime(IcalDateTime::TZ { tzid, date }) => {
                if tzid != expected_tzid {
                    bail!("TZ mismatch")
                }
                dates.push(ToNaivePeriod {
                    start: date,
                    duration,
                })
            }
            _ => bail!("DateTime mismatch"),
        }
    }

    Ok(dates)
}

#[derive(Debug, Clone)]
pub struct OffsetRule {
    pub offset_from: FixedOffset,
    pub offset_to: FixedOffset,
    pub start: NaiveDateTime,
    pub recur: Option<RecurRule>,
    pub name: Option<String>,
    pub rdates: Vec<NaiveDateTime>,
    pub exdates: Vec<NaiveDateTime>,
    pub properties: Vec<Property>,
}

impl TryFrom<parser::Component> for OffsetRule {
    type Error = Error;

    fn try_from(component: parser::Component) -> Result<Self, Self::Error> {
        ensure!(
            &component.name.to_ascii_uppercase() == "DAYLIGHT"
                || &component.name.to_ascii_uppercase() == "STANDARD"
        );

        if !component.sub_components.is_empty() {
            bail!("Neither DAYLIGHT nor STANDARD can have sub components");
        }

        let mut offset_from = None;
        let mut offset_to = None;
        let mut start = None;
        let mut recur = None;
        let mut name = None;

        let mut rdates = Vec::new();
        let mut exdates = Vec::new();

        let mut properties = Vec::new();
        for prop in component.properties {
            let parsed: Property = prop.try_into()?;

            match parsed {
                Property::TimeZoneOffsetFrom(value) => offset_from = Some(value.value),
                Property::TimeZoneOffsetTo(value) => offset_to = Some(value.value),
                Property::Start(value) => {
                    if let DateOrDateTime::DateTime(IcalDateTime::Local(datetime)) = value.value {
                        start = Some(datetime)
                    } else {
                        bail!("Invalid timezone start time, must be local time")
                    }
                }
                Property::RecurrenceRule(value) => recur = Some(value.value),
                Property::TimeZoneName(value) => name = Some(value.value),
                Property::RecurrenceDateTimes(value) => {
                    if let DateDateTimeOrPeriod::DateTime(IcalDateTime::Local(d)) = value.value {
                        rdates.push(d)
                    } else {
                        bail!(
                            "Unexpected type for RDATE in {}",
                            component.name.to_ascii_uppercase()
                        )
                    }
                }
                Property::ExceptionDateTimes(value) => {
                    if let DateOrDateTime::DateTime(IcalDateTime::Local(d)) = value.value {
                        exdates.push(d)
                    } else {
                        bail!(
                            "Unexpected type for EXDATE in {}",
                            component.name.to_ascii_uppercase()
                        )
                    }
                }
                p => properties.push(p),
            }
        }

        Ok(OffsetRule {
            offset_from: offset_from
                .ok_or_else(|| format_err!("Missing TZOFFSETFROM field in offset rule"))?,
            offset_to: offset_to
                .ok_or_else(|| format_err!("Missing TZOFFSETTO field in offset rule"))?,
            start: start.ok_or_else(|| format_err!("Missing DTSTART field in offset rule"))?,
            recur,
            rdates,
            exdates,
            name,
            properties,
        })
    }
}

#[derive(Debug, Clone)]
pub struct VTimeZone {
    pub id: String,
    pub standard: Vec<OffsetRule>,
    pub daylight: Vec<OffsetRule>,

    pub properties: Vec<Property>,
}

impl TryFrom<parser::Component> for VTimeZone {
    type Error = Error;

    fn try_from(component: parser::Component) -> Result<Self, Self::Error> {
        ensure!(component.name.to_ascii_uppercase() == "VTIMEZONE");

        let mut standard = Vec::new();
        let mut daylight = Vec::new();
        for component in component.sub_components {
            match &component.name.to_ascii_uppercase() as &str {
                "STANDARD" => standard.push(component.try_into()?),
                "DAYLIGHT" => daylight.push(component.try_into()?),
                _ => {} // TODO: Handle other components
            }
        }

        if standard.is_empty() && daylight.is_empty() {
            bail!("VTIMEZONE must have one of DAYLIGHT or STANDARD components");
        }

        let mut id = None;

        let mut properties = Vec::new();
        for prop in component.properties {
            let parsed: Property = prop.try_into()?;

            match parsed {
                Property::TimeZoneID(value) => id = Some(value.value),
                p => properties.push(p),
            }
        }

        Ok(VTimeZone {
            id: id.ok_or_else(|| format_err!("Missing TZID field in offset rule"))?,
            standard,
            daylight,
            properties,
        })
    }
}

impl VTimeZone {
    /// Find the offset for the given date. Date should either be in local time,
    /// or at UTC.
    pub fn get_offset(&self, date: NaiveDateTime, local: bool) -> FixedOffset {
        let effective_standard = get_effective_offset(&self.standard, date, local);
        let effective_daylight = get_effective_offset(&self.daylight, date, local);

        match (effective_standard, effective_daylight) {
            (Some(standard), Some(daylight)) => {
                // We iterate over recurrence until we find a period that matches.
                let last_standard_before = if let Some(recur) = &standard.recur {
                    recur
                        .from_date_with_extras(
                            standard.start,
                            standard.rdates.iter().cloned(),
                            &standard.exdates,
                            standard.offset_from,
                        )
                        .take_while(|&d| {
                            d <= if local {
                                date
                            } else {
                                date + standard.offset_from
                            }
                        })
                        .last()
                        .unwrap_or(standard.start)
                } else {
                    standard.start
                };

                let last_daylight_before = if let Some(recur) = &daylight.recur {
                    recur
                        .from_date(daylight.start, &daylight.offset_from)
                        .take_while(|&d| {
                            d <= if local {
                                date
                            } else {
                                date + daylight.offset_from
                            }
                        })
                        .last()
                        .unwrap_or(daylight.start)
                } else {
                    daylight.start
                };

                if last_daylight_before < last_standard_before {
                    standard.offset_to
                } else {
                    daylight.offset_to
                }
            }
            (Some(standard), None) => standard.offset_to,
            (None, Some(daylight)) => daylight.offset_to,
            (None, None) => panic!("invalid timezone"), // TODO: don't panic
        }
    }
}

impl Offseter for VTimeZone {
    fn to_instance(&self, d: NaiveDateTime) -> DateTime<FixedOffset> {
        self.get_offset(d, true)
            .from_local_datetime(&d)
            .earliest()
            .expect("valid timezone date")
    }

    fn from_instance(&self, d: DateTime<FixedOffset>) -> NaiveDateTime {
        d.naive_utc() + self.get_offset(d.naive_utc(), false)
    }
}

fn get_effective_offset(
    slice: &[OffsetRule],
    date: NaiveDateTime,
    local: bool,
) -> Option<OffsetRule> {
    let mut effective = None;

    for slice in slice.windows(2) {
        let (from, upto) = (&slice[0], &slice[1]);

        let date = if local { date } else { date + from.offset_from };

        if from.start <= date && date < upto.start {
            if let Some(recur) = &from.recur {
                // If there is a recur rule with an end condition (*must* be
                // UntilUtc) then we need to check that the date is valid.
                if let EndCondition::UntilUtc(until) = &recur.end_condition {
                    let offset_time = from
                        .offset_from
                        .from_local_datetime(&date)
                        .earliest()
                        .expect("valid datetime"); // This can't fail in FixedOffset
                    if *until < offset_time {
                        continue;
                    }
                }
            }
            effective = Some(from.clone());
            break;
        }
    }

    if let Some(last) = slice.last() {
        let date = if local { date } else { date + last.offset_from };

        if effective.is_none() && last.start <= date {
            effective = Some(last.clone());

            if let Some(recur) = &last.recur {
                // If there is a recur rule with an end condition (*must* be
                // UntilUtc) then we need to check that the date is valid.
                if let EndCondition::UntilUtc(until) = &recur.end_condition {
                    let offset_time = last
                        .offset_from
                        .from_local_datetime(&date)
                        .earliest()
                        .expect("valid datetime"); // This can't fail in FixedOffset

                    if *until < offset_time {
                        effective = None;
                    }
                }
            }
        }
    }

    effective
}

#[derive(Debug, Clone)]
pub struct EventCollection {
    pub base_event: VEvent,
    overrides: HashMap<DateOrDateTime, VEvent>,
}

impl EventCollection {
    fn new(events: Vec<VEvent>) -> Result<EventCollection, Error> {
        let mut base_event = None;
        let mut overrides = HashMap::new();

        let event_id = events.first().map(|e| e.uid.clone()).unwrap_or_default();

        for event in events {
            if !event.is_recurrence_instance {
                base_event = Some(event);
            } else if let Some(timings) = &event.timings {
                let date = match timings {
                    Timings::Date(d) => d.recur_id.map(DateOrDateTime::Date),
                    Timings::Local(_) => todo!(),
                    Timings::Utc(d) => d
                        .recur_id
                        .map(|d| DateOrDateTime::DateTime(IcalDateTime::Utc(d))),
                    Timings::Tz { tzid, inner } => inner.recur_id.map(|d| {
                        DateOrDateTime::DateTime(IcalDateTime::TZ {
                            tzid: tzid.clone(),
                            date: d,
                        })
                    }),
                    Timings::PerioidDate(d) => d.recur_id.map(DateOrDateTime::Date),
                    Timings::PerioidLocal(_) => todo!(),
                    Timings::PerioidUtc(d) => d
                        .recur_id
                        .map(|d| DateOrDateTime::DateTime(IcalDateTime::Utc(d))),
                    Timings::PerioidTz { tzid, inner } => inner.recur_id.map(|d| {
                        DateOrDateTime::DateTime(IcalDateTime::TZ {
                            tzid: tzid.clone(),
                            date: d,
                        })
                    }),
                };

                if let Some(date) = date {
                    overrides.insert(date, event);
                }
            }
        }

        let base_event = base_event.with_context(|| format!("missing base event: {}", event_id))?;

        Ok(EventCollection {
            base_event,
            overrides,
        })
    }

    pub fn recur_iter<'a>(
        &'a self,
        calendar: &'a VCalendar,
    ) -> Result<impl Iterator<Item = (DateTime<FixedOffset>, &'a VEvent)> + 'a, Error> {
        let mut overrides: BTreeMap<_, _> = self
            .overrides
            .iter()
            .filter_map(|(d, event)| {
                // TODO: Others?
                if let DateOrDateTime::DateTime(d) = d {
                    Some(calendar.get_time(d).map(|d| (d, event)))
                } else {
                    // Should we error (or panic?) if we find items of different
                    // types here?
                    None
                }
            })
            .collect::<Result<_, _>>()?;

        let exceptions: BTreeSet<_> = overrides.keys().copied().collect();

        let mut to_remove = exceptions.clone();
        if let Some(max_date) = exceptions.iter().max() {
            for date in self.base_event.recur_iter(calendar)? {
                if max_date < &date {
                    break;
                }

                to_remove.remove(&date);
            }
        }

        for date in to_remove {
            overrides.remove(&date);
        }

        let base_iter = self
            .base_event
            .recur_iter(calendar)?
            .filter(move |date| !exceptions.contains(date))
            .map(move |date| (date, &self.base_event));

        // TODO: Handle the case of recurrence ID being THISANDFUTURE?

        let exception_iters: Vec<_> = overrides
            .into_iter()
            .map(|(_, v)| {
                v.recur_iter(calendar)
                    .map(|iter| iter.map(move |date| (date, v)))
            })
            .collect::<Result<_, Error>>()?;

        let exception_iter = exception_iters.into_iter().kmerge_by(|a, b| a.0 < b.0);

        Ok(Box::new(
            base_iter.merge_by(exception_iter, |a, b| a.0 < b.0),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_naive_date(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    #[test]
    fn simple_london() {
        let timezone = VTimeZone {
            id: "Europe/London".to_string(),
            daylight: vec![OffsetRule {
                offset_from: FixedOffset::east(0),
                offset_to: FixedOffset::east(3600),
                start: make_naive_date("1981-03-29 01:00:00"),
                recur: Some("FREQ=YEARLY;BYDAY=-1SU;BYMONTH=3".parse().unwrap()),
                name: Some("BST".to_string()),
                rdates: vec![],
                exdates: vec![],
                properties: vec![],
            }],
            standard: vec![OffsetRule {
                offset_from: FixedOffset::east(3600),
                offset_to: FixedOffset::east(0),
                start: make_naive_date("1996-10-27 02:00:00"),
                recur: Some("FREQ=YEARLY;BYDAY=-1SU;BYMONTH=10".parse().unwrap()),
                name: Some("GMT".to_string()),
                rdates: vec![],
                exdates: vec![],
                properties: vec![],
            }],
            properties: vec![],
        };

        assert_eq!(
            timezone.get_offset(make_naive_date("2020-08-23 00:00:00"), true),
            FixedOffset::east(3600)
        );

        assert_eq!(
            timezone.get_offset(make_naive_date("2020-01-01 00:00:00"), true),
            FixedOffset::east(0)
        );
    }

    #[test]
    fn test_new_york() {
        let timezone = VTimeZone {
            id: "America/New_York".to_string(),
            daylight: vec![
                OffsetRule {
                    offset_from: FixedOffset::west(5 * 3600),
                    offset_to: FixedOffset::west(4 * 3600),
                    start: make_naive_date("1987-04-05 02:00:00"),
                    recur: Some(
                        "FREQ=YEARLY;BYMONTH=4;BYDAY=1SU;UNTIL=20060402T070000Z"
                            .parse()
                            .unwrap(),
                    ),
                    name: Some("EDT".to_string()),
                    rdates: vec![],
                    exdates: vec![],
                    properties: vec![],
                },
                OffsetRule {
                    offset_from: FixedOffset::west(5 * 3600),
                    offset_to: FixedOffset::west(4 * 3600),
                    start: make_naive_date("2007-03-11 02:00:00"),
                    recur: Some("FREQ=YEARLY;BYMONTH=3;BYDAY=2SU".parse().unwrap()),
                    name: Some("EDT".to_string()),
                    rdates: vec![],
                    exdates: vec![],
                    properties: vec![],
                },
            ],
            standard: vec![
                OffsetRule {
                    offset_from: FixedOffset::west(4 * 3600),
                    offset_to: FixedOffset::west(5 * 3600),
                    start: make_naive_date("1967-10-29 02:00:00"),
                    recur: Some(
                        "FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU;UNTIL=20061029T060000Z"
                            .parse()
                            .unwrap(),
                    ),
                    name: Some("EST".to_string()),
                    rdates: vec![],
                    exdates: vec![],
                    properties: vec![],
                },
                OffsetRule {
                    offset_from: FixedOffset::west(4 * 3600),
                    offset_to: FixedOffset::west(5 * 3600),
                    start: make_naive_date("2007-11-04 02:00:00"),
                    recur: Some("FREQ=YEARLY;BYMONTH=11;BYDAY=1SU".parse().unwrap()),
                    name: Some("EST".to_string()),
                    rdates: vec![],
                    exdates: vec![],
                    properties: vec![],
                },
            ],
            properties: vec![],
        };

        assert_eq!(
            timezone.get_offset(make_naive_date("1997-11-01 00:00:00"), true),
            FixedOffset::west(5 * 3600)
        );

        assert_eq!(
            timezone.get_offset(make_naive_date("1998-07-23 00:00:00"), true),
            FixedOffset::west(4 * 3600)
        );

        assert_eq!(
            timezone.get_offset(make_naive_date("1998-01-01 00:00:00"), true),
            FixedOffset::west(5 * 3600)
        );

        assert_eq!(
            timezone.get_offset(make_naive_date("2020-07-23 00:00:00"), true),
            FixedOffset::west(4 * 3600)
        );

        assert_eq!(
            timezone.get_offset(make_naive_date("2020-01-01 00:00:00"), true),
            FixedOffset::west(5 * 3600)
        );

        assert_eq!(
            timezone.get_offset(make_naive_date("1998-01-01 00:00:00"), true),
            FixedOffset::west(5 * 3600)
        );
    }

    #[test]
    fn parse_vcalendar() {
        let input = include_str!("../example.ics");

        let mut components = parser::Component::from_str_to_stream(input).unwrap();

        assert!(components.len() == 1);
        let calendar: VCalendar = components.pop().unwrap().try_into().unwrap();

        // Check that the first three times are expected
        let event = &calendar.events.values().next().unwrap().base_event;
        let times = event
            .recur_iter(&calendar)
            .unwrap()
            .take(3)
            .collect::<Vec<_>>();
        let expected_times: Vec<_> = vec![
            "2020-07-22T14:00:00+01:00",
            "2020-08-05T14:00:00+01:00",
            "2020-08-19T14:00:00+01:00",
        ]
        .into_iter()
        .map(|s| s.parse::<DateTime<FixedOffset>>().unwrap())
        .collect();

        assert_eq!(times, expected_times);

        // Check the times around clock changes are correct.
        let iter = event.recur_iter(&calendar).unwrap().skip(6);
        let times = iter.take(3).collect::<Vec<_>>();
        let expected_times: Vec<_> = vec![
            "2020-10-14T14:00:00+01:00",
            "2020-10-28T14:00:00+00:00",
            "2020-11-11T14:00:00+00:00",
        ]
        .into_iter()
        .map(|s| s.parse::<DateTime<FixedOffset>>().unwrap())
        .collect();

        assert_eq!(times, expected_times);

        // Test that iterating over periods work.
        let iter = event.recur_period_iter(&calendar).unwrap().skip(6);
        let times = iter.take(3).collect::<Vec<_>>();
        let expected_times: Vec<_> = vec![
            "2020-10-14T14:00:00+01:00",
            "2020-10-28T14:00:00+00:00",
            "2020-11-11T14:00:00+00:00",
        ]
        .into_iter()
        .map(|s| s.parse::<DateTime<FixedOffset>>().unwrap())
        .map(|start| ToNaivePeriod {
            start,
            duration: Duration::minutes(50),
        })
        .collect();

        assert_eq!(times, expected_times);
    }
}
