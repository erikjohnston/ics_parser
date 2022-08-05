use std::{collections::VecDeque, convert::TryFrom, fmt::Debug, ops::Add, str::FromStr};

use crate::{components::VCalendar, unescape::unescape};
use anyhow::{bail, format_err, Context, Error};
use chrono::{
    Date, DateTime, Datelike, Duration, FixedOffset, NaiveDate, NaiveDateTime, TimeZone, Timelike,
    Utc, Weekday,
};
use itertools::Itertools;
use url::Url;

use crate::{parameters::ParameterSet, parser};

#[derive(Debug, Clone)]
pub enum Property {
    /// Purpose:  This PropertyValue provides the capability to associate a document
    /// object with a calendar component.
    ///
    /// PropertyValue Parameters:  IANA, non-standard, inline encoding, and value
    /// data type PropertyValue parameters can be specified on this PropertyValue. The
    /// format type parameter can be specified on this PropertyValue and is
    /// RECOMMENDED for inline binary encoded content information.
    ///
    /// Conformance:  This PropertyValue can be specified multiple times in a
    /// "VEVENT", "VTODO", "VJOURNAL", or "VALARM" calendar component with the
    /// exception of AUDIO alarm that only allows this PropertyValue to occur once.
    ///
    /// Description:  This PropertyValue is used in "VEVENT", "VTODO", and "VJOURNAL"
    /// calendar components to associate a resource (e.g., document) with the
    /// calendar component.  This PropertyValue is used in "VALARM" calendar
    /// components to specify an audio sound resource or an email message
    /// attachment.  This PropertyValue can be specified as a URI pointing to a
    /// resource or as inline binary encoded content.
    ///
    /// When this PropertyValue is specified as inline binary encoded content,
    /// calendar applications MAY attempt to guess the media type of the
    /// resource via inspection of its content if and only if the media type of
    /// the resource is not given by the "FMTTYPE" parameter.  If the media type
    /// remains unknown, calendar applications SHOULD treat it as type
    /// "application/octet-stream".
    Attach(PropertyValue<AttachEnum>),

    /// Purpose:  This PropertyValue defines the access classification for a calendar
    /// component.
    ///
    /// PropertyValue Parameters:  IANA and non-standard PropertyValue parameters can be
    /// specified on this PropertyValue.
    ///
    /// Conformance:  The PropertyValue can be specified once in a "VEVENT", "VTODO",
    /// or "VJOURNAL" calendar components.
    ///
    /// Description:  An access classification is only one component of the
    /// general security system within a calendar application.  It provides a
    /// method of capturing the scope of the access the calendar owner intends
    /// for information within an individual calendar entry.  The access
    /// classification of an individual iCalendar component is useful when
    /// measured along with the other security components of a calendar system
    /// (e.g., calendar user authentication, authorization, access rights,
    /// access role, etc.). Hence, the semantics of the individual access
    /// classifications cannot be completely defined by this memo alone.
    /// Additionally, due to the "blind" nature of most exchange processes using
    /// this memo, these access classifications cannot serve as an enforcement
    /// statement for a system receiving an iCalendar object.  Rather, they
    /// provide a method for capturing the intention of the calendar owner for
    /// the access to the calendar component.  If not specified in a component
    /// that allows this PropertyValue, the default value is PUBLIC.  Applications
    /// MUST treat x-name and iana-token values they don't recognize the same
    /// way as they would the PRIVATE value.
    Categories(PropertyValue<Vec<String>>),

    /// Purpose:  This property defines the access classification for a calendar
    /// component.
    ///
    /// Conformance:  The property can be specified once in a "VEVENT", "VTODO",
    /// or "VJOURNAL" calendar components.
    ///
    /// Description:  An access classification is only one component of the
    /// general security system within a calendar application.  It provides a
    /// method of capturing the scope of the access the calendar owner intends
    /// for information within an individual calendar entry.  The access
    /// classification of an individual iCalendar component is useful when
    /// measured along with the other security components of a calendar system
    /// (e.g., calendar user authentication, authorization, access rights,
    /// access role, etc.). Hence, the semantics of the individual access
    /// classifications cannot be completely defined by this memo alone.
    /// Additionally, due to the "blind" nature of most exchange processes using
    /// this memo, these access classifications cannot serve as an enforcement
    /// statement for a system receiving an iCalendar object.  Rather, they
    /// provide a method for capturing the intention of the calendar owner for
    /// the access to the calendar component.  If not specified in a component
    /// that allows this property, the default value is PUBLIC.  Applications
    /// MUST treat x-name and iana-token values they don't recognize the same
    /// way as they would the PRIVATE value.
    Class(PropertyValue<ClassEnum>),

    /// Purpose:  This PropertyValue specifies non-processing information intended to
    /// provide a comment to the calendar user.
    ///
    /// PropertyValue Parameters:  IANA, non-standard, alternate text representation,
    /// and language PropertyValue parameters can be specified on this PropertyValue.
    ///
    /// Conformance:  This PropertyValue can be specified multiple times in "VEVENT",
    /// "VTODO", "VJOURNAL", and "VFREEBUSY" calendar components as well as in
    /// the "STANDARD" and "DAYLIGHT" sub-components.
    ///
    /// Description:  This PropertyValue is used to specify a comment to the calendar
    /// user.
    Comment(PropertyValue<String>),

    /// Purpose:  This PropertyValue provides a more complete description of the
    /// calendar component than that provided by the "SUMMARY" PropertyValue.
    ///
    /// PropertyValue Parameters:  IANA, non-standard, alternate text representation,
    /// and language PropertyValue parameters can be specified on this PropertyValue.
    ///
    /// Conformance:  The PropertyValue can be specified in the "VEVENT", "VTODO",
    /// "VJOURNAL", or "VALARM" calendar components.  The PropertyValue can be
    /// specified multiple times only within a "VJOURNAL" calendar component.
    ///
    /// Description:  This PropertyValue is used in the "VEVENT" and "VTODO" to
    /// capture lengthy textual descriptions associated with the activity.
    ///
    /// This PropertyValue is used in the "VJOURNAL" calendar component to capture
    /// one or more textual journal entries.
    ///
    /// This PropertyValue is used in the "VALARM" calendar component to capture the
    /// display text for a DISPLAY category of alarm, and to capture the body
    /// text for an EMAIL category of alarm.
    Description(PropertyValue<String>),

    /// Purpose:  This PropertyValue specifies information related to the global
    /// position for the activity specified by a calendar component.
    ///
    /// PropertyValue Parameters:  IANA and non-standard PropertyValue parameters can be
    /// specified on this PropertyValue.
    ///
    /// Conformance:  This PropertyValue can be specified in "VEVENT" or "VTODO"
    /// calendar components.
    ///
    /// Description:  This PropertyValue value specifies latitude and longitude, in
    /// that order (i.e., "LAT LON" ordering).  The longitude represents the
    /// location east or west of the prime meridian as a positive or negative
    /// real number, respectively.  The longitude and latitude values MAY be
    /// specified up to six decimal places, which will allow for accuracy to
    /// within one meter of geographical position.  Receiving applications MUST
    /// accept values of this precision and MAY truncate values of greater
    /// precision.
    ///
    /// Values for latitude and longitude shall be expressed as decimal
    /// fractions of degrees.  Whole degrees of latitude shall be represented by
    /// a two-digit decimal number ranging from 0 through 90. Whole degrees of
    /// longitude shall be represented by a decimal number ranging from 0
    /// through 180.  When a decimal fraction of a degree is specified, it shall
    /// be separated from the whole number of degrees by a decimal point.
    ///
    /// Latitudes north of the equator shall be specified by a plus sign (+), or
    /// by the absence of a minus sign (-), preceding the digits designating
    /// degrees.  Latitudes south of the Equator shall be designated by a minus
    /// sign (-) preceding the digits designating degrees.  A point on the
    /// Equator shall be assigned to the Northern Hemisphere.
    ///
    /// Longitudes east of the prime meridian shall be specified by a plus sign
    /// (+), or by the absence of a minus sign (-), preceding the digits
    /// designating degrees.  Longitudes west of the meridian shall be
    /// designated by minus sign (-) preceding the digits designating degrees. A
    /// point on the prime meridian shall be assigned to the Eastern Hemisphere.
    /// A point on the 180th meridian shall be assigned to the Western
    /// Hemisphere.  One exception to this last convention is permitted. For the
    /// special condition of describing a band of latitude around the earth, the
    /// East Bounding Coordinate data element shall be assigned the value +180
    /// (180) degrees.
    ///
    /// Any spatial address with a latitude of +90 (90) or -90 degrees will
    /// specify the position at the North or South Pole, respectively.  The
    /// component for longitude may have any legal value.
    ///
    /// With the exception of the special condition described above, this form
    /// is specified in [ANSI INCITS 61-1986].
    ///
    /// The simple formula for converting degrees-minutes-seconds into decimal
    /// degrees is:
    ///
    /// ```notest
    ///     decimal = degrees + minutes/60 + seconds/3600.
    /// ```
    Geo(PropertyValue<(f64, f64)>),

    /// Purpose:  This PropertyValue defines the intended venue for the activity
    /// defined by a calendar component.
    ///
    /// PropertyValue Parameters:  IANA, non-standard, alternate text representation,
    /// and language PropertyValue parameters can be specified on this PropertyValue.
    ///
    /// Conformance:  This PropertyValue can be specified in "VEVENT" or "VTODO"
    /// calendar component.
    ///
    /// Description:  Specific venues such as conference or meeting rooms may be
    /// explicitly specified using this PropertyValue.  An alternate representation
    /// may be specified that is a URI that points to directory information with
    /// more structured specification of the location.  For example, the
    /// alternate representation may specify either an LDAP URL RFC4516 pointing
    /// to an LDAP server entry or a CID URL RFC239 pointing to a MIME body part
    /// containing a Virtual-Information Card (vCard) RFC2426 for the location.
    Location(PropertyValue<String>),

    PercentComplete(PropertyValue<u32>),
    Priority(PropertyValue<u32>),
    Resources(PropertyValue<Vec<String>>),
    Status(PropertyValue<StatusEnum>),
    Summary(PropertyValue<String>),

    Completed(PropertyValue<DateTime<Utc>>),
    End(PropertyValue<DateOrDateTime>),
    Due(PropertyValue<DateOrDateTime>),
    Start(PropertyValue<DateOrDateTime>),
    Duration(PropertyValue<Duration>),
    FreeBusyTime(PropertyValue<Vec<Period>>),
    Transparency(PropertyValue<TransparencyEnum>),

    TimeZoneID(PropertyValue<String>),
    TimeZoneName(PropertyValue<String>),
    TimeZoneOffsetFrom(PropertyValue<FixedOffset>),
    TimeZoneOffsetTo(PropertyValue<FixedOffset>),
    TimeZoneURL(PropertyValue<Url>),

    Attendee(PropertyValue<Url>),
    Contact(PropertyValue<String>),
    Organizer(PropertyValue<Url>),
    RecurrenceID(PropertyValue<DateOrDateTime>),
    RelatedTo(PropertyValue<String>),
    URL(PropertyValue<Url>),
    UID(PropertyValue<String>),

    ExceptionDateTimes(PropertyValue<DateOrDateTime>),
    RecurrenceDateTimes(PropertyValue<DateDateTimeOrPeriod>),
    RecurrenceRule(PropertyValue<RecurRule>),

    Action(PropertyValue<String>),
    Repeat(PropertyValue<u32>),
    Trigger(PropertyValue<DateTimeOrDuration>),

    Created(PropertyValue<DateTime<Utc>>),
    DateTimeStamp(PropertyValue<DateTime<Utc>>),
    LastModified(PropertyValue<DateTime<Utc>>),
    SequenceNumber(PropertyValue<u32>),

    RequestStatus(PropertyValue<RequestStatus>),

    ProductIdentifier(PropertyValue<String>),
    Version(PropertyValue<String>),

    // TODO: Add the others
    Other(String, PropertyValue<String>),
}

impl TryFrom<parser::Property> for Property {
    type Error = Error;

    fn try_from(property: parser::Property) -> Result<Self, Self::Error> {
        let parameters: ParameterSet = property.parameters.iter().cloned().into();

        let prop = match &property.name.to_ascii_uppercase() as &str {
            "ATTACH" => {
                if let Some(data_type) = parameters.get_value_data_type() {
                    match &data_type.to_ascii_uppercase() as &str {
                        "BINARY" => {
                            if parameters.get_encoding() != Some("BASE64") {
                                bail!("Unknown encoding for binary attach property");
                            }

                            let value = base64::decode(&property.value)?;
                            Property::Attach(PropertyValue {
                                value: AttachEnum::Binary(value),
                                parameters,
                            })
                        }
                        "URI" => Property::Attach(PropertyValue {
                            value: AttachEnum::Url(property.value.parse()?),
                            parameters,
                        }),
                        data_type => Property::Attach(PropertyValue {
                            value: AttachEnum::Other {
                                data_type: data_type.to_string(),
                                value: property.value.clone(),
                            },
                            parameters,
                        }),
                    }
                } else {
                    Property::Attach(PropertyValue {
                        value: AttachEnum::Url(property.value.parse()?),
                        parameters,
                    })
                }
            }
            "CATEGORIES" => Property::Categories(PropertyValue {
                value: property
                    .value
                    .split(',')
                    .map(|s| unescape(&s.trim().to_string()))
                    .collect::<Result<_, _>>()?,
                parameters,
            }),
            // "CLASS" => todo!(),
            "COMMENT" => Property::Comment(PropertyValue {
                value: unescape(&property.value)?,
                parameters,
            }),
            "DESCRIPTION" => Property::Description(PropertyValue {
                value: unescape(&property.value)?,
                parameters,
            }),
            // "GEO" => todo!(),
            "LOCATION" => Property::Location(PropertyValue {
                value: unescape(&property.value)?,
                parameters,
            }),
            "PERCENT-COMPLETE" => Property::PercentComplete(PropertyValue {
                value: property.value.parse()?,
                parameters,
            }),
            "PRIORITY" => Property::Priority(PropertyValue {
                value: property.value.parse()?,
                parameters,
            }),
            "RESOURCES" => Property::Resources(PropertyValue {
                value: property
                    .value
                    .split(',')
                    .map(|s| unescape(&s.trim().to_string()))
                    .collect::<Result<_, _>>()?,
                parameters,
            }),
            // "STATUS" => todo!(),
            "SUMMARY" => Property::Summary(PropertyValue {
                value: unescape(&property.value)?,
                parameters,
            }),
            // "COMPLETED" => todo!(),
            "DTEND" => Property::End(PropertyValue {
                value: DateOrDateTime::parse_from(&property.value, &parameters)?,
                parameters,
            }),
            "DUE" => Property::Due(PropertyValue {
                value: DateOrDateTime::parse_from(&property.value, &parameters)?,
                parameters,
            }),
            "DTSTART" => Property::Start(PropertyValue {
                value: DateOrDateTime::parse_from(&property.value, &parameters)?,
                parameters,
            }),
            // "DURATION" => todo!(),
            // "FREEBUSY" => todo!(),
            "TRANSP" => {
                let value = match &property.value.to_ascii_uppercase() as &str {
                    "OPAQUE" => TransparencyEnum::Opaque,
                    "TRANSPARENT" => TransparencyEnum::Tranparent,
                    _ => TransparencyEnum::Other(property.value.clone()),
                };
                Property::Transparency(PropertyValue { value, parameters })
            }
            "TZID" => Property::TimeZoneID(PropertyValue {
                value: unescape(&property.value)?,
                parameters,
            }),
            "TZNAME" => Property::TimeZoneName(PropertyValue {
                value: unescape(&property.value)?,
                parameters,
            }),
            "TZOFFSETFROM" => Property::TimeZoneOffsetFrom(PropertyValue {
                value: parse_offset(&property.value)?,
                parameters,
            }),
            "TZOFFSETTO" => Property::TimeZoneOffsetTo(PropertyValue {
                value: parse_offset(&property.value)?,
                parameters,
            }),
            "TZURL" => Property::TimeZoneURL(PropertyValue {
                value: property.value.parse()?,
                parameters,
            }),
            "ATTENDEE" => Property::Attendee(PropertyValue {
                value: property.value.parse()?,
                parameters,
            }),
            "CONTACT" => Property::Contact(PropertyValue {
                value: unescape(&property.value)?,
                parameters,
            }),
            "ORGANIZER" => Property::Organizer(PropertyValue {
                value: property.value.parse()?,
                parameters,
            }),
            "RECURRENCE-ID" => Property::RecurrenceID(PropertyValue {
                value: DateOrDateTime::parse_from(&property.value, &parameters)?,
                parameters,
            }),
            "RELATED-TO" => Property::RelatedTo(PropertyValue {
                value: unescape(&property.value)?,
                parameters,
            }),
            "URL" => Property::URL(PropertyValue {
                value: property.value.parse()?,
                parameters,
            }),
            "UID" => Property::UID(PropertyValue {
                value: unescape(&property.value)?,
                parameters,
            }),
            "EXDATE" => Property::ExceptionDateTimes(PropertyValue {
                value: DateOrDateTime::parse_from(&property.value, &parameters)?,
                parameters,
            }),
            "RDATE" => Property::RecurrenceDateTimes(PropertyValue {
                value: DateDateTimeOrPeriod::parse_from(&property.value, &parameters)?,
                parameters,
            }),
            "RRULE" => Property::RecurrenceRule(PropertyValue {
                value: property.value.parse()?,
                parameters,
            }),
            "ACTION" => Property::Action(PropertyValue {
                value: unescape(&property.value)?,
                parameters,
            }),
            "REPEAT" => Property::Repeat(PropertyValue {
                value: property.value.parse()?,
                parameters,
            }),
            // "TRIGGER" => todo!(),
            "CREATED" => {
                let date = DateOrDateTime::parse_from(&property.value, &parameters)?;

                if let DateOrDateTime::DateTime(IcalDateTime::Utc(date)) = date {
                    Property::Created(PropertyValue {
                        value: date,
                        parameters,
                    })
                } else {
                    bail!("CREATED must be UTC")
                }
            }
            "DTSTAMP" => {
                let date = DateOrDateTime::parse_from(&property.value, &parameters)?;

                if let DateOrDateTime::DateTime(IcalDateTime::Utc(date)) = date {
                    Property::DateTimeStamp(PropertyValue {
                        value: date,
                        parameters,
                    })
                } else {
                    bail!("DTSTAMP must be UTC")
                }
            }
            // "LAST-MODIFIED" => {
            //     let date = DateOrDateTime::parse_from(&property.value, &parameters)?;

            //     if let DateOrDateTime::DateTime(IcalDateTime::Utc(date)) = date {
            //         Property::LastModified(PropertyValue {
            //             value: date,
            //             parameters,
            //         })
            //     } else {
            //         bail!("LAST-MODIFIED must be UTC")
            //     }
            // }
            "SEQUENCE" => Property::SequenceNumber(PropertyValue {
                value: property.value.parse()?,
                parameters,
            }),
            // "REQUEST-STATUS" => todo!(),
            "PRODID" => Property::ProductIdentifier(PropertyValue {
                value: property.value.parse()?,
                parameters,
            }),
            "VERSION" => Property::Version(PropertyValue {
                value: property.value.parse()?,
                parameters,
            }),
            _ => Property::Other(
                property.name,
                PropertyValue {
                    value: property.value,
                    parameters,
                },
            ),
        };

        Ok(prop)
    }
}

fn parse_offset(value: &str) -> Result<FixedOffset, Error> {
    if !value.starts_with(&['+', '-'] as &[char]) || value.len() != 5 {
        bail!("Invalid TZOFFSETFROM prop: {}", value)
    }
    let hours: i32 = value[1..3].parse()?;
    let seconds: i32 = value[3..].parse()?;

    if value.starts_with('+') {
        Ok(FixedOffset::east(hours * 60 * 60 + seconds))
    } else {
        Ok(FixedOffset::west(hours * 60 * 60 + seconds))
    }
}

#[derive(Debug, Clone)]
pub struct PropertyValue<T: Debug + Clone> {
    pub value: T,
    pub parameters: ParameterSet,
}

#[derive(Debug, Clone)]
pub enum AttachEnum {
    Url(Url),
    Binary(Vec<u8>),
    Other { data_type: String, value: String },
}

#[derive(Debug, Clone)]
pub enum ClassEnum {
    Public,
    Private,
    Confidential,
    Other(String),
}

#[derive(Debug, Clone)]
pub enum StatusEnum {
    Cancelled,

    Tentative,
    Confirmed,

    NeesAction,
    Completed,
    InProgress,

    Draft,
    Final,

    Other(String),
}

#[derive(Debug, Clone)]
pub enum DateDateTimeOrPeriod {
    Date(NaiveDate),
    DateTime(IcalDateTime),
    Period(Period),
}

impl DateDateTimeOrPeriod {
    fn parse_from(value: &str, params: &ParameterSet) -> Result<Self, Error> {
        if let Ok(period) = Period::parse_from(value, params) {
            Ok(DateDateTimeOrPeriod::Period(period))
        } else {
            Ok(match DateOrDateTime::parse_from(value, params)? {
                DateOrDateTime::Date(d) => DateDateTimeOrPeriod::Date(d),
                DateOrDateTime::DateTime(d) => DateDateTimeOrPeriod::DateTime(d),
            })
        }
    }
}

impl TryFrom<DateDateTimeOrPeriod> for NaiveDate {
    type Error = Error;

    fn try_from(value: DateDateTimeOrPeriod) -> Result<Self, Self::Error> {
        match value {
            DateDateTimeOrPeriod::Date(d) => Ok(d),
            _ => bail!("Not a date"),
        }
    }
}

impl TryFrom<DateDateTimeOrPeriod> for NaiveDateTime {
    type Error = Error;

    fn try_from(value: DateDateTimeOrPeriod) -> Result<Self, Self::Error> {
        match value {
            DateDateTimeOrPeriod::DateTime(IcalDateTime::Local(d)) => Ok(d),
            _ => bail!("Not a date"),
        }
    }
}

impl TryFrom<DateOrDateTime> for DateTime<Utc> {
    type Error = Error;

    fn try_from(value: DateOrDateTime) -> Result<Self, Self::Error> {
        match value {
            DateOrDateTime::DateTime(IcalDateTime::Utc(d)) => Ok(d),
            _ => bail!("Not a date"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DateTimeOrDuration {
    DateTime(IcalDateTime),
    Duration(Duration),
}

#[derive(Debug, Clone)]
pub struct Period {
    pub start: IcalDateTime,
    pub duration: Duration,
}

impl Period {
    fn parse_from(value: &str, params: &ParameterSet) -> Result<Self, Error> {
        let (start, end) = value.split_once('/').context("invalid period")?;

        println!("start {}, end {}", start, end);

        let start = match DateOrDateTime::parse_from(start, params)? {
            DateOrDateTime::Date(_) => bail!("Invalid start time in period"),
            DateOrDateTime::DateTime(d) => d,
        };

        let r = regex::Regex::new("(-?P)(?:T?([0-9]+)([WDHMS]))+").unwrap();

        if let Some(captures) = r.captures(end) {
            let mut iter = captures.iter();
            iter.next(); // skip full match
            let period = iter
                .next()
                .expect("regex failed to find group")
                .expect("regex failed to find group");

            let mut duration = Duration::seconds(0);

            for (digits, length) in iter.tuples() {
                let duration_value: i64 = digits
                    .expect("regex failed to find group")
                    .as_str()
                    .parse()?;

                let duration_part = match length.expect("regex failed to find group").as_str() {
                    "W" => Duration::weeks(duration_value),
                    "D" => Duration::days(duration_value),
                    "H" => Duration::hours(duration_value),
                    "M" => Duration::minutes(duration_value),
                    "S" => Duration::seconds(duration_value),
                    _ => bail!("invalid period duration"),
                };

                duration = duration_part + duration;
            }

            if period.as_str() == "-P" {
                duration = duration * -1;
            }

            Ok(Period { start, duration })
        } else {
            let end = match DateOrDateTime::parse_from(end, params)? {
                DateOrDateTime::Date(_) => bail!("Invalid start time in period"),
                DateOrDateTime::DateTime(d) => d,
            };

            let duration = end.sub(&start, None)?;

            Ok(Period { start, duration })
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IcalDateTime {
    Local(NaiveDateTime),
    Utc(DateTime<Utc>),
    TZ { date: NaiveDateTime, tzid: String },
}

impl IcalDateTime {
    /// Return the duration between to IcalDateTime.
    ///
    /// If a `vcalendar` is parsed then it can correctly calculate the duration
    /// between times with different timezones. If its not passed in then it
    /// errors.
    pub fn sub(
        &self,
        other: &IcalDateTime,
        vcalendar: Option<&VCalendar>,
    ) -> Result<Duration, Error> {
        match (self, other) {
            (IcalDateTime::Local(left), IcalDateTime::Local(right)) => return Ok(*left - *right),
            (IcalDateTime::Utc(left), IcalDateTime::Utc(right)) => return Ok(*left - *right),
            (
                IcalDateTime::TZ {
                    date: left,
                    tzid: left_tzid,
                },
                IcalDateTime::TZ {
                    date: right,
                    tzid: right_tzid,
                },
            ) => {
                if left_tzid == right_tzid {
                    return Ok(*left - *right);
                }
            }
            _ => {}
        }

        let cal = vcalendar.context("Mismatched IcalDateTime")?;

        let left: DateTime<FixedOffset>;
        let right: DateTime<FixedOffset>;

        match self {
            IcalDateTime::Utc(t) => left = t.with_timezone(&FixedOffset::east(0)),
            IcalDateTime::TZ { .. } => {
                left = cal.get_time(self)?;
            }
            IcalDateTime::Local(_) => bail!("Mismatched IcalDateTime"),
        }

        match other {
            IcalDateTime::Utc(t) => right = t.with_timezone(&FixedOffset::east(0)),
            IcalDateTime::TZ { .. } => {
                right = cal.get_time(self)?;
            }
            IcalDateTime::Local(_) => bail!("Mismatched IcalDateTime"),
        }

        Ok(left - right)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DateOrDateTime {
    Date(NaiveDate),
    DateTime(IcalDateTime),
}

impl DateOrDateTime {
    fn parse_from(value: &str, params: &ParameterSet) -> Result<Self, Error> {
        if value.contains('T') {
            if value.ends_with('Z') {
                Ok(DateOrDateTime::DateTime(IcalDateTime::Utc(
                    Utc.datetime_from_str(value, "%Y%m%dT%H%M%SZ")?,
                )))
            } else if let Some(tzid) = params.get_tzid() {
                Ok(DateOrDateTime::DateTime(IcalDateTime::TZ {
                    date: NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S")?,
                    tzid: tzid.to_string(),
                }))
            } else {
                Ok(DateOrDateTime::DateTime(IcalDateTime::Local(
                    NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S")
                        .context(format!("value: {}", value))?,
                )))
            }
        } else {
            Ok(DateOrDateTime::Date(NaiveDate::parse_from_str(
                value, "%Y%m%d",
            )?))
        }
    }
}

impl TryFrom<DateOrDateTime> for NaiveDate {
    type Error = Error;

    fn try_from(value: DateOrDateTime) -> Result<Self, Self::Error> {
        match value {
            DateOrDateTime::Date(d) => Ok(d),
            _ => bail!("Not a date"),
        }
    }
}

impl TryFrom<DateOrDateTime> for NaiveDateTime {
    type Error = Error;

    fn try_from(value: DateOrDateTime) -> Result<Self, Self::Error> {
        match value {
            DateOrDateTime::DateTime(IcalDateTime::Local(d)) => Ok(d),
            _ => bail!("Not a date"),
        }
    }
}

impl TryFrom<DateDateTimeOrPeriod> for DateTime<Utc> {
    type Error = Error;

    fn try_from(value: DateDateTimeOrPeriod) -> Result<Self, Self::Error> {
        match value {
            DateDateTimeOrPeriod::DateTime(IcalDateTime::Utc(d)) => Ok(d),
            _ => bail!("Not a date"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum TransparencyEnum {
    Opaque,
    Tranparent,
    Other(String),
}

#[derive(Debug, Clone)]
pub struct RequestStatus {
    code: u16,
    description: String,
    data: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Frequency {
    Secondly,
    Minutely,
    Hourly,
    Daily,
    Weekly,
    Monthly,
    Yearly,
}

impl Frequency {
    /// Create a date that has been advanced by the frequency the given number
    /// of times.
    ///
    /// Note for years and dates the day gets reset to 1 (as not all days are
    /// valid for all years and months).
    pub fn advance_date<T: ExtendedDatelike>(self, date: T, interval: u64) -> T {
        match self {
            Frequency::Secondly => date + Duration::seconds(interval as i64),
            Frequency::Minutely => date + Duration::minutes(interval as i64),
            Frequency::Hourly => date + Duration::hours(interval as i64),
            Frequency::Daily => date + Duration::days(interval as i64),
            Frequency::Weekly => date + Duration::days(7 * interval as i64),
            Frequency::Monthly => {
                // Chrono doesn't currently have a way of adding months, c.f.
                // chronotope/chrono#474.

                let current_month = date.month0();

                date.with_day(1)
                    .expect("valid month")
                    .with_month0((current_month + interval as u32) % 12)
                    .expect("valid month")
                    .with_year(date.year() + (current_month + interval as u32) as i32 / 12)
                    .expect("valid month")
            }
            Frequency::Yearly => date
                .with_day(1)
                .expect("valid month")
                .with_year(date.year() + interval as i32)
                .expect("year increment"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EndCondition {
    Count(u64),
    Until(NaiveDateTime),
    UntilUtc(DateTime<Utc>), // TODO: Add date
    Infinite,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecurRule {
    pub frequency: Frequency,
    pub interval: u64,
    pub end_condition: EndCondition,
    pub by_second: Vec<u8>,
    pub by_minute: Vec<u8>,
    pub by_hour: Vec<u8>,
    pub by_day: Vec<(Option<i8>, Weekday)>,
    pub by_month_day: Vec<i8>,
    pub by_year_day: Vec<i16>,
    pub by_week_number: Vec<i16>,
    pub by_month: Vec<u16>,
    pub by_set_pos: Vec<i16>,
    pub week_start: Weekday,
}

pub trait Offseter {
    fn to_instance(&self, d: NaiveDateTime) -> DateTime<FixedOffset>;
    fn from_instance(&self, d: DateTime<FixedOffset>) -> NaiveDateTime;
}

impl Offseter for FixedOffset {
    fn to_instance(&self, d: NaiveDateTime) -> DateTime<FixedOffset> {
        self.from_local_datetime(&d)
            .earliest()
            .expect("valid timezone date")
    }

    fn from_instance(&self, d: DateTime<FixedOffset>) -> NaiveDateTime {
        d.naive_utc() + *self
    }
}

impl RecurRule {
    pub fn from_date(
        &self,
        date: NaiveDateTime,
        offseter: &dyn Offseter,
    ) -> impl Iterator<Item = NaiveDateTime> {
        let (max_count, until) = match self.end_condition {
            EndCondition::Count(c) => (Some(c), None),
            EndCondition::Until(t) => (None, Some(t)),
            EndCondition::UntilUtc(t) => (None, Some(offseter.from_instance(t.into()))),
            _ => (None, None),
        };

        RecurIter {
            recur: self.clone(),
            next_date: Some(date),
            queue: VecDeque::new(),
            count: 0,
            max_count,
            until,
            previous_date: None,
        }
    }

    pub fn from_naive_date_with_extras<
        'a,
        T: ToNaive + 'a,
        E,
        O: Offseter + 'a,
        I: IntoIterator<Item = T::Naive> + 'a,
    >(
        &self,
        date: T::Naive,
        rdates: I,
        exdates: &'a [E],
        offseter: O,
    ) -> impl Iterator<Item = T> + 'a
    where
        T::Naive: PartialEq<E>,
    {
        let (max_count, until) = match self.end_condition {
            EndCondition::Count(c) => (Some(c), None),
            EndCondition::Until(t) => (None, Some(t)),
            EndCondition::UntilUtc(t) => (
                None,
                Some(
                    offseter
                        .from_instance(t.into())
                        .to_naive()
                        .to_naive_datetime(),
                ),
            ),
            _ => (None, None),
        };

        let iter = RecurIter {
            recur: self.clone(),
            next_date: Some(date.to_naive()),
            queue: VecDeque::new(),
            count: 0,
            max_count,
            until,
            previous_date: None,
        };

        iter.merge(rdates)
            .filter(move |d| exdates.iter().all(|ex| !d.eq(ex)))
            .dedup()
            .map(move |d| T::from_naive(d, &offseter))
    }

    pub fn from_date_with_extras<
        'a,
        T: ToNaive + 'a,
        E,
        O: Offseter + 'a,
        I: IntoIterator<Item = T> + 'a,
    >(
        &self,
        date: T,
        rdates: I,
        exdates: &'a [E],
        offseter: O,
    ) -> impl Iterator<Item = T> + 'a
    where
        T: PartialEq<E>,
        T::Naive: PartialEq,
    {
        let (max_count, until) = match self.end_condition {
            EndCondition::Count(c) => (Some(c), None),
            EndCondition::Until(t) => (None, Some(t)),
            EndCondition::UntilUtc(t) => (
                None,
                Some(
                    offseter
                        .from_instance(t.into())
                        .to_naive()
                        .to_naive_datetime(),
                ),
            ),
            _ => (None, None),
        };

        let iter = RecurIter {
            recur: self.clone(),
            next_date: Some(date.to_naive()),
            queue: VecDeque::new(),
            count: 0,
            max_count,
            until,
            previous_date: None,
        };

        iter.map(move |d| T::from_naive(d, &offseter))
            .merge(rdates)
            .dedup()
            .filter(move |d| exdates.iter().all(|ex| !d.eq(ex)))
    }
}

impl FromStr for RecurRule {
    type Err = Error;

    fn from_str(rule_value_string: &str) -> Result<Self, Self::Err> {
        let mut frequency = None;
        let mut interval = 1;
        let mut end_condition = EndCondition::Infinite;
        let mut by_second = Vec::new();
        let mut by_minute = Vec::new();
        let mut by_hour = Vec::new();
        let mut by_day = Vec::new();
        let mut by_month_day = Vec::new();
        let mut by_year_day = Vec::new();
        let mut by_week_number = Vec::new();
        let mut by_month = Vec::new();
        let mut by_set_pos = Vec::new();
        let mut week_start = Weekday::Mon;

        for part in rule_value_string.split(';') {
            let split_pos = part
                .find('=')
                .ok_or_else(|| format_err!("Invalid recur rule: '{}'", part))?;
            let (name, tail) = part.split_at(split_pos);
            let value = &tail[1..];

            match &name.to_ascii_uppercase() as &str {
                "FREQ" => {
                    frequency = Some(match &value.to_ascii_uppercase() as &str {
                        "SECONDLY" => Frequency::Secondly,
                        "MINUTELY" => Frequency::Minutely,
                        "HOURLY" => Frequency::Hourly,
                        "DAILY" => Frequency::Daily,
                        "WEEKLY" => Frequency::Weekly,
                        "MONTHLY" => Frequency::Monthly,
                        "YEARLY" => Frequency::Yearly,
                        _ => bail!("Invalid frequency: '{}'", value),
                    });
                }
                "UNTIL" => {
                    end_condition = if value.contains('T') {
                        if value.ends_with('Z') {
                            let parsed = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%SZ")
                                .with_context(|| format!("Invalid recur rule date: {}", part))?;
                            EndCondition::UntilUtc(DateTime::from_utc(parsed, Utc))
                        } else {
                            let parsed = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S")
                                .with_context(|| format!("Invalid recur rule date: {}", part))?;
                            EndCondition::Until(parsed)
                        }
                    } else {
                        let parsed = NaiveDateTime::parse_from_str(value, "%Y%m%d")
                            .with_context(|| format!("Invalid recur rule date: {}", part))?;
                        EndCondition::Until(parsed)
                    }
                }
                "COUNT" => {
                    end_condition = EndCondition::Count(
                        value
                            .parse::<u64>()
                            .with_context(|| format!("Invalid recur rule option: {}", part))?,
                    )
                }
                "INTERVAL" => {
                    interval = value
                        .parse::<u64>()
                        .with_context(|| format!("Invalid recur rule option: {}", part))?
                }
                "BYSECOND" => {
                    by_second = value
                        .split(',')
                        .map(|s| s.parse::<u8>())
                        .collect::<Result<Vec<_>, _>>()
                        .with_context(|| format!("Invalid recur rule option: {}", part))?;

                    // Ensure that seconds are in the appropriate range
                    for s in &by_second {
                        if !(0..=60).contains(s) {
                            bail!("Invalid recur rule option: {}", part)
                        }
                    }
                }
                "BYMINUTE" => {
                    by_minute = value
                        .split(',')
                        .map(|s| s.parse::<u8>())
                        .collect::<Result<Vec<_>, _>>()
                        .with_context(|| format!("Invalid recur rule option: {}", part))?;

                    // Ensure that minutes are in the appropriate range
                    for s in &by_minute {
                        if !(0..=60).contains(s) {
                            bail!("Invalid recur rule option: {}", part)
                        }
                    }
                }
                "BYHOUR" => {
                    by_hour = value
                        .split(',')
                        .map(|s| s.parse::<u8>())
                        .collect::<Result<Vec<_>, _>>()
                        .with_context(|| format!("Invalid recur rule option: {}", part))?;

                    // Ensure that hours are in the appropriate range
                    for s in &by_hour {
                        if !(0..=24).contains(s) {
                            bail!("Invalid recur rule option: {}", part)
                        }
                    }
                }
                "BYDAY" => {
                    for val in value.split_terminator(',') {
                        let re = regex::Regex::new(r"^([+-]?[0-9]+)")?;
                        let num = if let Some(mat) = re.find(val) {
                            Some(mat.as_str().parse()?)
                        } else {
                            None
                        };

                        let val = val.to_ascii_uppercase();

                        let weekday = if val.ends_with("MO") {
                            Weekday::Mon
                        } else if val.ends_with("TU") {
                            Weekday::Tue
                        } else if val.ends_with("WE") {
                            Weekday::Wed
                        } else if val.ends_with("TH") {
                            Weekday::Thu
                        } else if val.ends_with("FR") {
                            Weekday::Fri
                        } else if val.ends_with("SA") {
                            Weekday::Sat
                        } else if val.ends_with("SU") {
                            Weekday::Sun
                        } else {
                            bail!("Invalid recur rule option: {}", part)
                        };

                        by_day.push((num, weekday));
                    }
                }
                "BYMONTHDAY" => {
                    by_month_day = value
                        .split(',')
                        .map(|s| s.parse::<i8>())
                        .collect::<Result<Vec<_>, _>>()
                        .with_context(|| format!("Invalid recur rule option: {}", part))?;

                    // Ensure that hours are in the appropriate range
                    for s in &by_month_day {
                        if !(1..=31).contains(&s.abs()) {
                            bail!("Invalid recur rule option: {}", part)
                        }
                    }
                }
                "BYYEARDAY" => {
                    by_year_day = value
                        .split(',')
                        .map(|s| s.parse::<i16>())
                        .collect::<Result<Vec<_>, _>>()
                        .with_context(|| format!("Invalid recur rule option: {}", part))?;

                    // Ensure that hours are in the appropriate range
                    for s in &by_year_day {
                        if !(1..=366).contains(&s.abs()) {
                            bail!("Invalid recur rule option: {}", part)
                        }
                    }
                }
                "BYWEEKNO" => {
                    by_week_number = value
                        .split(',')
                        .map(|s| s.parse::<i16>())
                        .collect::<Result<Vec<_>, _>>()
                        .with_context(|| format!("Invalid recur rule option: {}", part))?;

                    // Ensure that hours are in the appropriate range
                    for s in &by_week_number {
                        if !(1..=53).contains(&s.abs()) {
                            bail!("Invalid recur rule option: {}", part)
                        }
                    }
                }
                "BYMONTH" => {
                    by_month = value
                        .split(',')
                        .map(|s| s.parse::<u16>())
                        .collect::<Result<Vec<_>, _>>()
                        .with_context(|| format!("Invalid recur rule option: {}", part))?;

                    // Ensure that hours are in the appropriate range
                    for s in &by_month {
                        if !(1..=12).contains(s) {
                            bail!("Invalid recur rule option: {}", part)
                        }
                    }
                }
                "BYSETPOS" => {
                    by_set_pos = value
                        .split(',')
                        .map(|s| s.parse::<i16>())
                        .collect::<Result<Vec<_>, _>>()
                        .with_context(|| format!("Invalid recur rule option: {}", part))?;

                    // Ensure that hours are in the appropriate range
                    for s in &by_set_pos {
                        if !(1..=366).contains(&s.abs()) {
                            bail!("Invalid recur rule option: {}", part)
                        }
                    }
                }
                "WKST" => {
                    week_start = match &value.to_ascii_uppercase() as &str {
                        "MO" => Weekday::Mon,
                        "TU" => Weekday::Tue,
                        "WE" => Weekday::Wed,
                        "TH" => Weekday::Thu,
                        "FR" => Weekday::Fri,
                        "SA" => Weekday::Sat,
                        "SU" => Weekday::Sun,
                        _ => bail!("Invalid recur rule option: {}", part),
                    };
                }
                _ => bail!("Invalid recur rule option: '{}'", part),
            }
        }

        let frequency = frequency.ok_or_else(|| format_err!("Missing FREQ in RRULE"))?;

        if !by_week_number.is_empty() && frequency != Frequency::Yearly {
            bail!(
                "Invalid recur rule combination: cannot combine BYWEEKNO with non-YEARLY frequency"
            );
        }

        if !by_year_day.is_empty()
            && [Frequency::Daily, Frequency::Weekly, Frequency::Monthly].contains(&frequency)
        {
            bail!(
                "Invalid recur rule combination: cannot combine BYYEARDAY with DAILY/WEEKLY/MONTHLY frequency"
            );
        }

        if !by_month_day.is_empty() && frequency == Frequency::Weekly {
            bail!(
                "Invalid recur rule combination: cannot combine BYMONTHDAY with WEEKLY frequency"
            );
        }

        if frequency != Frequency::Monthly && frequency != Frequency::Yearly {
            for (i, _) in &by_day {
                if i.is_some() {
                    bail!("Invalid recur rule combination: cannot have integer in BYDAY when frequency is not MONTHLY or YEARLY")
                }
            }
        }

        Ok(RecurRule {
            frequency,
            interval,
            end_condition,
            by_second,
            by_minute,
            by_hour,
            by_day,
            by_month_day,
            by_year_day,
            by_week_number,
            by_month,
            by_set_pos,
            week_start,
        })
    }
}

pub trait ExtendedDatelike: Datelike + Add<Duration, Output = Self> + PartialOrd + Copy {
    fn same_day(&self, other: &Self) -> bool {
        self.year() == other.year() && self.ordinal() == other.ordinal()
    }
}
pub trait ExtendedDateTimelike: Timelike + ExtendedDatelike {}

impl<T> ExtendedDatelike for T where T: Datelike + Add<Duration, Output = Self> + PartialOrd + Copy {}
impl<T> ExtendedDateTimelike for T where T: Timelike + ExtendedDatelike {}

pub trait Expandable: Sized + PartialOrd + Copy {
    fn expand_date_set(&self, recur: &RecurRule) -> Vec<Self>;
    fn advance(self, frequency: Frequency, interval: u64) -> Self;

    fn less_than_or_equal_local_datetime(&self, d: NaiveDateTime) -> bool;
    fn to_naive_datetime(&self) -> NaiveDateTime;
}

impl Expandable for NaiveDate {
    fn expand_date_set(&self, recur: &RecurRule) -> Vec<Self> {
        expand_dates(recur, vec![*self])
    }

    fn advance(self, frequency: Frequency, interval: u64) -> Self {
        frequency
            .advance_date(self.and_hms(0, 0, 0), interval)
            .date()
    }

    fn less_than_or_equal_local_datetime(&self, d: NaiveDateTime) -> bool {
        *self <= d.date()
    }

    fn to_naive_datetime(&self) -> NaiveDateTime {
        self.and_hms(23, 59, 59)
    }
}

impl Expandable for NaiveDateTime {
    fn expand_date_set(&self, recur: &RecurRule) -> Vec<Self> {
        let date_set = expand_dates(recur, vec![*self]);
        expand_times(recur, date_set)
    }

    fn advance(self, frequency: Frequency, interval: u64) -> Self {
        frequency.advance_date(self, interval)
    }

    fn less_than_or_equal_local_datetime(&self, d: NaiveDateTime) -> bool {
        *self <= d
    }

    fn to_naive_datetime(&self) -> NaiveDateTime {
        *self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NaivePeriod<T: Expandable> {
    duration: Duration,
    start: T,
}

impl<T: Expandable> PartialOrd for NaivePeriod<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.start.partial_cmp(&other.start)
    }
}

impl<T: Expandable> PartialEq<T> for NaivePeriod<T> {
    fn eq(&self, other: &T) -> bool {
        self.start.eq(other)
    }
}

impl<T: Expandable> Expandable for NaivePeriod<T> {
    fn expand_date_set(&self, recur: &RecurRule) -> Vec<Self> {
        let date_set = self.start.expand_date_set(recur);

        let duration = self.duration;
        date_set
            .into_iter()
            .map(|start| NaivePeriod { start, duration })
            .collect()
    }

    fn advance(self, frequency: Frequency, interval: u64) -> Self {
        NaivePeriod {
            duration: self.duration,
            start: self.start.advance(frequency, interval),
        }
    }

    fn less_than_or_equal_local_datetime(&self, d: NaiveDateTime) -> bool {
        self.start.less_than_or_equal_local_datetime(d)
    }

    fn to_naive_datetime(&self) -> NaiveDateTime {
        self.start.to_naive_datetime()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToNaivePeriod<T: ToNaive> {
    pub duration: Duration,
    pub start: T,
}

impl<T: ToNaive> PartialOrd for ToNaivePeriod<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.start.partial_cmp(&other.start)
    }
}

impl<T: ToNaive> PartialEq<T> for ToNaivePeriod<T> {
    fn eq(&self, other: &T) -> bool {
        self.start.eq(other)
    }
}

impl<T: ToNaive> ToNaive for ToNaivePeriod<T> {
    type Naive = NaivePeriod<T::Naive>;

    fn to_naive(&self) -> Self::Naive {
        NaivePeriod {
            start: self.start.to_naive(),
            duration: self.duration,
        }
    }

    fn from_naive(naive: Self::Naive, offseter: &dyn Offseter) -> Self {
        ToNaivePeriod {
            start: T::from_naive(naive.start, offseter),
            duration: naive.duration,
        }
    }
}

pub trait ToNaive: PartialOrd + Copy {
    type Naive: Expandable + Debug;

    fn to_naive(&self) -> Self::Naive;
    fn from_naive(naive: Self::Naive, offseter: &dyn Offseter) -> Self;
}

impl<T> ToNaive for T
where
    T: Expandable + Debug,
{
    type Naive = T;

    fn to_naive(&self) -> Self::Naive {
        *self
    }

    fn from_naive(naive: Self::Naive, _: &dyn Offseter) -> Self {
        naive
    }
}

impl ToNaive for Date<FixedOffset> {
    type Naive = NaiveDate;

    fn to_naive(&self) -> Self::Naive {
        self.naive_local()
    }

    fn from_naive(naive: Self::Naive, offseter: &dyn Offseter) -> Self {
        offseter.to_instance(naive.and_hms(0, 0, 0)).date()
    }
}

impl ToNaive for DateTime<FixedOffset> {
    type Naive = NaiveDateTime;

    fn to_naive(&self) -> Self::Naive {
        self.naive_local()
    }

    fn from_naive(naive: Self::Naive, offseter: &dyn Offseter) -> Self {
        offseter.to_instance(naive)
    }
}

impl ToNaive for DateTime<Utc> {
    type Naive = NaiveDateTime;

    fn to_naive(&self) -> Self::Naive {
        self.naive_local()
    }

    fn from_naive(naive: Self::Naive, offseter: &dyn Offseter) -> Self {
        offseter.to_instance(naive).with_timezone(&Utc)
    }
}

pub struct RecurIter<T> {
    recur: RecurRule,
    next_date: Option<T>,
    queue: VecDeque<T>,
    max_count: Option<u64>,
    until: Option<NaiveDateTime>,
    count: u64,
    previous_date: Option<T>,
}

impl<T> Iterator for RecurIter<T>
where
    T: Expandable + PartialEq,
{
    type Item = T;

    fn next(&mut self) -> Option<T> {
        while self.queue.is_empty() {
            let curr_date = self.next_date.take()?;

            self.next_date = Some(curr_date.advance(self.recur.frequency, self.recur.interval));

            let mut date_set = curr_date.expand_date_set(&self.recur);

            if !self.recur.by_set_pos.is_empty() {
                date_set = self
                    .recur
                    .by_set_pos
                    .iter()
                    .copied()
                    .map(|p| {
                        if p > 0 {
                            p - 1
                        } else {
                            p.rem_euclid(self.recur.by_set_pos.len() as i16)
                        }
                    })
                    .map(|pos| date_set[pos as usize])
                    .collect();
            }

            if !date_set.is_empty() {
                self.queue = date_set
                    .into_iter()
                    .filter(|date| Some(*date) != self.previous_date)
                    .dedup()
                    .collect();
            }
        }

        if let Some(to_return) = self.queue.pop_front() {
            if let Some(max_count) = self.max_count {
                if self.count >= max_count {
                    return None;
                }
            }
            if let Some(until) = self.until {
                if !to_return.less_than_or_equal_local_datetime(until) {
                    return None;
                }
            }

            self.count += 1;

            self.previous_date = Some(to_return);

            return Some(to_return);
        }

        None
    }
}

fn expand_dates<T>(recur: &RecurRule, date_set: Vec<T>) -> Vec<T>
where
    T: ExtendedDatelike + Debug,
{
    let mut date_set = date_set;

    if !recur.by_month.is_empty() {
        match recur.frequency {
            Frequency::Secondly
            | Frequency::Minutely
            | Frequency::Hourly
            | Frequency::Daily
            | Frequency::Weekly
            | Frequency::Monthly => {
                date_set = date_set
                    .into_iter()
                    .filter(|d| recur.by_month.contains(&(d.month() as u16)))
                    .collect();
            }
            Frequency::Yearly => {
                date_set = date_set
                    .into_iter()
                    .flat_map(|d| {
                        recur
                            .by_month
                            .iter()
                            .map(move |&s| d.with_month(s as u32).expect("month expansion"))
                    })
                    .collect();
            }
        }
    }

    if !recur.by_week_number.is_empty() {
        match recur.frequency {
            Frequency::Secondly
            | Frequency::Minutely
            | Frequency::Hourly
            | Frequency::Daily
            | Frequency::Weekly
            | Frequency::Monthly => panic!("BYWEEKNO cannot be specified unless FREQ=YEARLY"),

            Frequency::Yearly => {
                let week_start = recur.week_start;

                date_set = date_set
                    .into_iter()
                    .flat_map(|d| {
                        recur.by_week_number.iter().copied().map(move |mut s| {
                            if s < 0 {
                                let weeks_in_year = get_weeks_in_year(week_start, d);
                                s %= weeks_in_year as i16;
                            }

                            let diff = s as u32 - d.iso_week().week();

                            d + Duration::days(diff as i64)
                        })
                    })
                    .collect();
            }
        }
    }

    if !recur.by_year_day.is_empty() {
        match recur.frequency {
            Frequency::Secondly
            | Frequency::Minutely
            | Frequency::Hourly
            | Frequency::Daily
            | Frequency::Weekly
            | Frequency::Monthly => {
                date_set = date_set
                    .into_iter()
                    .filter(|&d| {
                        let days_in_year = get_days_in_year(d) as i16;

                        let by_year_day: Vec<_> = recur
                            .by_year_day
                            .iter()
                            .map(|&s| if s > 0 { s - 1 } else { s + days_in_year })
                            .map(|s| (s % days_in_year) as u32 + 1)
                            .collect();

                        by_year_day.contains(&(d.ordinal() as u32))
                    })
                    .collect();
            }
            Frequency::Yearly => {
                date_set = date_set
                    .into_iter()
                    .flat_map(|d| {
                        let days_in_year = get_days_in_year(d) as i16;

                        recur
                            .by_year_day
                            .iter()
                            .map(move |&s| if s > 0 { s - 1 } else { s + days_in_year })
                            .map(move |s| (s % days_in_year as i16) as u32 + 1)
                            .map(move |s| d.with_ordinal(s).expect("year day expansion"))
                    })
                    .collect();
            }
        }
    }

    if !recur.by_month_day.is_empty() {
        match recur.frequency {
            Frequency::Secondly
            | Frequency::Minutely
            | Frequency::Hourly
            | Frequency::Daily
            | Frequency::Weekly => {
                date_set = date_set
                    .into_iter()
                    .filter(|&d| {
                        let days_in_month = get_days_in_month(d) as i8;

                        let by_month_day: Vec<_> = recur
                            .by_month_day
                            .iter()
                            .map(|&s| if s > 0 { s - 1 } else { s + days_in_month })
                            .map(|s| (s % days_in_month) as u32 + 1)
                            .collect();

                        by_month_day.contains(&d.day())
                    })
                    .collect();
            }

            Frequency::Monthly | Frequency::Yearly => {
                date_set = date_set
                    .into_iter()
                    .flat_map(|d| {
                        let days_in_month = get_days_in_month(d) as i8;

                        recur
                            .by_month_day
                            .iter()
                            .map(move |&s| if s > 0 { s - 1 } else { s + days_in_month })
                            .map(move |s| (s % days_in_month) as u32 + 1)
                            .map(move |s| d.with_day(s).expect("month day expansion"))
                    })
                    .collect()
            }
        }
    }

    if !recur.by_day.is_empty() {
        let is_month_day_empty = recur.by_month_day.is_empty();

        match recur.frequency {
            Frequency::Secondly | Frequency::Minutely | Frequency::Hourly | Frequency::Daily => {
                // We ignore the numeric part of BYDAY rule, as it can
                // only be set for MONTHLY or YEARLY.
                date_set = date_set
                    .into_iter()
                    .filter(|d| recur.by_day.iter().any(|&(_, day)| day == d.weekday()))
                    .collect();
            }
            Frequency::Weekly => {
                date_set = date_set
                    .into_iter()
                    .flat_map(|d| {
                        let week_start = get_start_of_week(recur.week_start, d);

                        recur.by_day.iter().map(move |&(_, day)| {
                            // We ignore the numeric part of BYDAY rule, as it can
                            // only be set for MONTHLY or YEARLY.

                            let diff = (7 + day.num_days_from_monday() as i64
                                - week_start.weekday().num_days_from_monday() as i64)
                                % 7; // We always want to go forwards;

                            week_start + Duration::days(diff)
                        })
                    })
                    .collect();
            }

            Frequency::Monthly => {
                date_set = date_set
                    .into_iter()
                    .flat_map(|d| {
                        let month_start = d.with_day(1).expect("month start");
                        let month_end = (month_start + Duration::days(32))
                            .with_day(1)
                            .expect("month_end");

                        recur.by_day.iter().flat_map(move |&(num, day)| {
                            let dates = get_weekdays_in_period(month_start, month_end, day, num);

                            // We limit if BYMONTHDAY is set. (We always
                            // add a filter clause so that types match)
                            dates
                                .into_iter()
                                .filter(move |date| is_month_day_empty || date.same_day(&d))
                        })
                    })
                    .collect();
            }

            Frequency::Yearly => {
                let limit = !recur.by_year_day.is_empty() || !recur.by_month_day.is_empty();

                let freq = if !recur.by_week_number.is_empty() {
                    Frequency::Weekly
                } else if !recur.by_month.is_empty() {
                    Frequency::Monthly
                } else {
                    Frequency::Yearly
                };
                let week_start = recur.week_start;

                date_set = date_set
                    .into_iter()
                    .flat_map(|d| {
                        let (start, end) = match freq {
                            Frequency::Weekly => {
                                let diff = d.weekday().num_days_from_monday()
                                    - week_start.num_days_from_monday();

                                let start = d + -Duration::days(diff as i64);

                                (start, start + Duration::days(7))
                            }
                            Frequency::Monthly => {
                                let month_start = d.with_day(1).expect("month start");
                                let month_end = (month_start + Duration::days(32))
                                    .with_day(1)
                                    .expect("month_end");

                                (month_start, month_end)
                            }
                            Frequency::Yearly => {
                                let start = d
                                    .with_day(1)
                                    .and_then(|d| d.with_month(1))
                                    .expect("year start");

                                (start, start.with_year(start.year() + 1).expect("year end"))
                            }
                            _ => unreachable!(),
                        };

                        recur.by_day.iter().flat_map(move |&(num, day)| {
                            let dates = get_weekdays_in_period(start, end, day, num);

                            // We limit if BYMONTHDAY is set. (We always
                            // add a filter clause so that types match)
                            dates
                                .into_iter()
                                .filter(move |date| !limit || date.same_day(&d))
                        })
                    })
                    .collect();
            }
        }
    }

    date_set
}

fn expand_times<T>(recur: &RecurRule, date_set: Vec<T>) -> Vec<T>
where
    T: ExtendedDateTimelike,
{
    let mut date_set = date_set;
    if !recur.by_hour.is_empty() {
        match recur.frequency {
            Frequency::Secondly | Frequency::Minutely | Frequency::Hourly => {
                date_set = date_set
                    .into_iter()
                    .filter(|d| recur.by_hour.contains(&(d.hour() as u8)))
                    .collect();
            }

            Frequency::Daily | Frequency::Weekly | Frequency::Monthly | Frequency::Yearly => {
                date_set = date_set
                    .into_iter()
                    .flat_map(|d| {
                        recur
                            .by_hour
                            .iter()
                            .map(move |&s| d.with_hour(s as u32).expect("hour expansion"))
                    })
                    .collect();
            }
        }
    }

    if !recur.by_minute.is_empty() {
        match recur.frequency {
            Frequency::Secondly | Frequency::Minutely => {
                date_set = date_set
                    .into_iter()
                    .filter(|d| recur.by_minute.contains(&(d.minute() as u8)))
                    .collect();
            }

            Frequency::Hourly
            | Frequency::Daily
            | Frequency::Weekly
            | Frequency::Monthly
            | Frequency::Yearly => {
                date_set = date_set
                    .into_iter()
                    .flat_map(|d| {
                        recur
                            .by_minute
                            .iter()
                            .map(move |&s| d.with_minute(s as u32).expect("minute expansion"))
                    })
                    .collect();
            }
        }
    }

    if !recur.by_second.is_empty() {
        match recur.frequency {
            Frequency::Secondly => {
                date_set = date_set
                    .into_iter()
                    .filter(|d| recur.by_second.contains(&(d.second() as u8)))
                    .collect();
            }

            Frequency::Minutely
            | Frequency::Hourly
            | Frequency::Daily
            | Frequency::Weekly
            | Frequency::Monthly
            | Frequency::Yearly => {
                date_set = date_set
                    .into_iter()
                    .flat_map(|d| {
                        recur
                            .by_second
                            .iter()
                            .map(move |&s| d.with_second(s as u32).expect("second expansion"))
                    })
                    .collect();
            }
        }
    }

    date_set
}

fn get_days_in_year<D: Datelike>(date: D) -> u32 {
    if date.with_ordinal(366).is_some() {
        366
    } else {
        365
    }
}

fn get_weeks_in_year<D: Datelike>(week_start: Weekday, date: D) -> u32 {
    if NaiveDate::from_isoywd_opt(date.year(), 53, week_start).is_some() {
        53
    } else {
        52
    }
}

fn get_days_in_month<D: Datelike>(date: D) -> u32 {
    for &days in &[31u32, 30, 29, 28] {
        if date.with_day(days).is_some() {
            return days;
        }
    }

    panic!("Days in month {} was not 31, 30, 29 or 28", date.month())
}

fn get_weekdays_in_period<T>(start: T, end: T, day: Weekday, num: Option<i8>) -> Vec<T>
where
    T: ExtendedDatelike,
{
    let mut potential_dates = Vec::with_capacity(5);
    let start_weekday = start.weekday();

    let diff = (day.num_days_from_monday() as i64 - start_weekday.num_days_from_monday() as i64)
        .rem_euclid(7); // We always want to go forwards

    let mut date = start + Duration::days(diff);

    // Now we add every occurence in this month.
    potential_dates.clear();
    while date < end {
        potential_dates.push(date);

        date = date + Duration::days(7);
    }
    if let Some(num) = num {
        if num > 0 {
            if let Some(&date) = potential_dates.get(num as usize - 1) {
                vec![date]
            } else {
                vec![]
            }
        } else if let Some(&date) =
            potential_dates.get((num as i32).rem_euclid(potential_dates.len() as i32) as usize)
        {
            vec![date]
        } else {
            vec![]
        }
    } else {
        // We add every specified weekday in the month.
        potential_dates
    }
}

/// Return the date of the start of the week.
fn get_start_of_week<T>(week_start: Weekday, date: T) -> T
where
    T: ExtendedDatelike,
{
    let mut difference =
        week_start.num_days_from_monday() as i64 - date.weekday().num_days_from_monday() as i64;

    // We always want the start of the week to be *before* the given date.
    if difference > 0 {
        difference = difference - 7;
    }

    let start_date = date + Duration::days(difference);

    assert_eq!(start_date.weekday(), week_start);

    start_date
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{OffsetRule, VTimeZone};

    #[test]
    fn parse_period() {
        Period::parse_from("20000101T000000/PT1H", &ParameterSet::default()).unwrap();
        Period::parse_from("20000101T000000/-PT1H", &ParameterSet::default()).unwrap();
        Period::parse_from("20000101T000000/P15DT5H0M20S", &ParameterSet::default()).unwrap();
        Period::parse_from("20000101T000000/20000101T010000", &ParameterSet::default()).unwrap();
    }

    #[test]
    fn test_advance_date() {
        // Test simple increment
        assert_eq!(
            Frequency::Secondly
                .advance_date::<NaiveDateTime>("2000-01-01T00:00:00".parse().unwrap(), 2),
            "2000-01-01T00:00:02".parse().unwrap()
        );
        assert_eq!(
            Frequency::Minutely
                .advance_date::<NaiveDateTime>("2000-01-01T00:00:00".parse().unwrap(), 2),
            "2000-01-01T00:02:00".parse().unwrap()
        );
        assert_eq!(
            Frequency::Hourly
                .advance_date::<NaiveDateTime>("2000-01-01T00:00:00".parse().unwrap(), 2),
            "2000-01-01T02:00:00".parse().unwrap()
        );
        assert_eq!(
            Frequency::Daily
                .advance_date::<NaiveDateTime>("2000-01-01T00:00:00".parse().unwrap(), 2),
            "2000-01-03T00:00:00".parse().unwrap()
        );
        assert_eq!(
            Frequency::Weekly
                .advance_date::<NaiveDateTime>("2000-01-01T00:00:00".parse().unwrap(), 2),
            "2000-01-15T00:00:00".parse().unwrap()
        );
        assert_eq!(
            Frequency::Monthly
                .advance_date::<NaiveDateTime>("2000-01-01T00:00:00".parse().unwrap(), 2),
            "2000-03-01T00:00:00".parse().unwrap()
        );
        assert_eq!(
            Frequency::Yearly
                .advance_date::<NaiveDateTime>("2000-01-01T00:00:00".parse().unwrap(), 2),
            "2002-01-01T00:00:00".parse().unwrap()
        );

        // Test wrap around
        assert_eq!(
            Frequency::Secondly
                .advance_date::<NaiveDateTime>("2000-01-01T00:00:59".parse().unwrap(), 2),
            "2000-01-01T00:01:01".parse().unwrap()
        );
        assert_eq!(
            Frequency::Minutely
                .advance_date::<NaiveDateTime>("2000-01-01T00:59:00".parse().unwrap(), 2),
            "2000-01-01T01:01:00".parse().unwrap()
        );
        assert_eq!(
            Frequency::Hourly
                .advance_date::<NaiveDateTime>("2000-01-01T23:00:00".parse().unwrap(), 2),
            "2000-01-02T01:00:00".parse().unwrap()
        );
        assert_eq!(
            Frequency::Daily
                .advance_date::<NaiveDateTime>("2000-01-31T00:00:00".parse().unwrap(), 2),
            "2000-02-02T00:00:00".parse().unwrap()
        );
        assert_eq!(
            Frequency::Weekly
                .advance_date::<NaiveDateTime>("2000-01-31T00:00:00".parse().unwrap(), 2),
            "2000-02-14T00:00:00".parse().unwrap()
        );
        assert_eq!(
            Frequency::Monthly
                .advance_date::<NaiveDateTime>("2000-12-01T00:00:00".parse().unwrap(), 2),
            "2001-02-01T00:00:00".parse().unwrap()
        );
        assert_eq!(
            Frequency::Monthly
                .advance_date::<NaiveDateTime>("2000-01-31T00:00:00".parse().unwrap(), 1),
            "2000-02-01T00:00:00".parse().unwrap()
        );
    }

    fn make_naive_date(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").unwrap()
    }

    fn timezone() -> VTimeZone {
        VTimeZone {
            id: "America/New_York".to_string(),
            daylight: vec![
                OffsetRule {
                    offset_from: FixedOffset::west(5 * 3600),
                    offset_to: FixedOffset::west(4 * 3600),
                    start: make_naive_date("1987-04-05 02:00:00"),
                    recur: Some(
                        RecurRule::from_str(
                            "FREQ=YEARLY;BYMONTH=4;BYDAY=1SU;UNTIL=20060402T070000Z",
                        )
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
                    recur: Some(RecurRule::from_str("FREQ=YEARLY;BYMONTH=3;BYDAY=2SU").unwrap()),
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
                        RecurRule::from_str(
                            "FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU;UNTIL=20061029T060000Z",
                        )
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
                    recur: Some(RecurRule::from_str("FREQ=YEARLY;BYMONTH=11;BYDAY=1SU").unwrap()),
                    name: Some("EST".to_string()),
                    rdates: vec![],
                    exdates: vec![],
                    properties: vec![],
                },
            ],
            properties: vec![],
        }
    }

    macro_rules! add_rrule_test {
        ($name:ident, $date:expr; test $string:expr => $test:expr) => {
            #[test]
            fn $name() {
                let date_start = DateTime::parse_from_rfc3339($date)
                    .map(|d| d.naive_local())
                    .or_else(|_| NaiveDateTime::parse_from_str($date, "%Y-%m-%dT%H:%M:%S"))
                    .unwrap();

                let rule = RecurRule::from_str($string).unwrap();

                $test(rule, date_start)
            }
        };

        ($name:ident, $date:expr; parse $string:expr => $expected:expr) => {
            add_rrule_test!($name, $date; test $string => |rule, _| {
                assert_eq!(rule, $expected);
            });
        };

        ($name:ident, $date:expr; finite $string:expr => $expected:expr) => {
            add_rrule_test!($name, $date; test $string => |rule: RecurRule, date_start: NaiveDateTime| {
                let timezone = timezone();

                let dates: Vec<_> = rule
                    .from_date(date_start, &timezone)
                    .map(|d| timezone.get_offset(d, true).from_local_datetime(&d).earliest().unwrap())
                    .map(|d| d.to_rfc3339().to_string())
                    .collect();

                let str_dates: Vec<_> = dates.iter().map(|s| s as &str).collect();

                assert_eq!(&str_dates, $expected);
            });
        };

        ($name:ident, $date:expr; infinite $string:expr => $expected:expr) => {
            add_rrule_test!($name, $date; test $string => |rule: RecurRule, date_start: NaiveDateTime| {
                let timezone = timezone();

                let dates: Vec<_> = rule
                    .from_date(date_start, &timezone)
                    .take($expected.len())
                    .map(|d| timezone.get_offset(d, true).from_local_datetime(&d).earliest().unwrap())
                    .map(|d| d.to_rfc3339().to_string())
                    .collect();

                let str_dates: Vec<_> = dates.iter().map(|s| s as &str).collect();
                assert_eq!(&str_dates, $expected);
            });
        };

        ($name:ident, $date:expr; pattern $string:expr => $expected:pat) => {
            add_rrule_test!($name, $date; test $string => |rule: RecurRule, date_start: NaiveDateTime| {
                let timezone = timezone();

                let dates: Vec<_> = rule
                    .from_date(date_start, &timezone)
                    .take(1000)
                    .map(|d| timezone.get_offset(d, true).from_local_datetime(&d).earliest().unwrap())
                    .map(|d| d.to_rfc3339().to_string())
                    .collect();

                let str_dates: Vec<_> = dates.iter().map(|s| s as &str).collect();

                assert!(matches!(
                    &str_dates as &[&str], $expected),
                    "got: {:#?}..{:#?}, expected: {}",
                    &str_dates[..5],
                    &str_dates[str_dates.len()-5..],
                    stringify!($expected)
                );
            });
        };

        ($name:ident, $date:expr; finite_naive $string:expr => $expected:expr) => {
            add_rrule_test!($name, $date; test $string => |rule: RecurRule, date_start: NaiveDateTime| {
                let timezone = timezone();

                let dates: Vec<_> = rule
                    .from_date(date_start, &timezone)
                    .map(|d| d.format("%Y-%m-%dT%H:%M:%S").to_string())
                    .collect();

                let str_dates: Vec<_> = dates.iter().map(|s| s as &str).collect();

                assert_eq!(&str_dates, $expected);
            });
        };
    }

    add_rrule_test! {
        recur_rule_parse_basic, "1997-01-05T13:30:00-04:00";
        parse "FREQ=WEEKLY;INTERVAL=1;BYDAY=WE" => RecurRule {
            frequency: Frequency::Weekly,
            interval: 1,
            end_condition: EndCondition::Infinite,
            by_second: vec![],
            by_minute: vec![],
            by_hour: vec![],
            by_day: vec![(None, Weekday::Wed)],
            by_month_day: vec![],
            by_year_day: vec![],
            by_week_number: vec![],
            by_month: vec![],
            by_set_pos: vec![],
            week_start: Weekday::Mon,
        }
    }

    add_rrule_test! {
        recur_rule_parse_complicated, "1997-01-05T08:30:00-04:00";
        parse "FREQ=YEARLY;INTERVAL=2;BYMONTH=1;BYDAY=SU;BYHOUR=8,9;BYMINUTE=30" => RecurRule {
            frequency: Frequency::Yearly,
            interval: 2,
            end_condition: EndCondition::Infinite,
            by_second: vec![],
            by_minute: vec![30],
            by_hour: vec![8, 9],
            by_day: vec![(None, Weekday::Sun)],
            by_month_day: vec![],
            by_year_day: vec![],
            by_week_number: vec![],
            by_month: vec![1],
            by_set_pos: vec![],
            week_start: Weekday::Mon,
        }
    }

    add_rrule_test! {
        recur_rule_parse_daily_for_10, "1997-09-02T09:00:00-04:00";
        parse "FREQ=DAILY;COUNT=10" => RecurRule {
            frequency: Frequency::Daily,
            interval: 1,
            end_condition: EndCondition::Count(10),
            by_second: vec![],
            by_minute: vec![],
            by_hour: vec![],
            by_day: vec![],
            by_month_day: vec![],
            by_year_day: vec![],
            by_week_number: vec![],
            by_month: vec![],
            by_set_pos: vec![],
            week_start: Weekday::Mon,
        }
    }

    add_rrule_test! {
        recur_rule_parse_daily_until_dec_24, "1997-09-02T09:00:00-04:00";
        parse "FREQ=DAILY;UNTIL=19971224T000000" => RecurRule {
            frequency: Frequency::Daily,
            interval: 1,
            end_condition: EndCondition::Until(NaiveDateTime::parse_from_str("1997-12-24 00:00:00", "%Y-%m-%d %H:%M:%S").unwrap()),
            by_second: vec![],
            by_minute: vec![],
            by_hour: vec![],
            by_day: vec![],
            by_month_day: vec![],
            by_year_day: vec![],
            by_week_number: vec![],
            by_month: vec![],
            by_set_pos: vec![],
            week_start: Weekday::Mon,
        }
    }

    add_rrule_test! {
        recur_rule_parse_every_other_day_forever, "1997-09-02T09:00:00-04:00";
        parse "FREQ=DAILY;INTERVAL=2" => RecurRule {
            frequency: Frequency::Daily,
            interval: 2,
            end_condition: EndCondition::Infinite,
            by_second: vec![],
            by_minute: vec![],
            by_hour: vec![],
            by_day: vec![],
            by_month_day: vec![],
            by_year_day: vec![],
            by_week_number: vec![],
            by_month: vec![],
            by_set_pos: vec![],
            week_start: Weekday::Mon,
        }
    }

    add_rrule_test! {
        recur_rule_parse_every_other_week_forever, "2022-07-26T10:00:00";
        parse "FREQ=WEEKLY;WKST=SU;INTERVAL=2;BYDAY=TU" => RecurRule {
            frequency: Frequency::Weekly,
            interval: 2,
            end_condition: EndCondition::Infinite,
            by_second: vec![],
            by_minute: vec![],
            by_hour: vec![],
            by_day: vec![(None, Weekday::Tue)],
            by_month_day: vec![],
            by_year_day: vec![],
            by_week_number: vec![],
            by_month: vec![],
            by_set_pos: vec![],
            week_start: Weekday::Sun,
        }
    }

    add_rrule_test! {
        recur_rule_iter_daily_for_10, "1997-09-02T09:00:00-04:00";
        finite "FREQ=DAILY;COUNT=10" => &[
            "1997-09-02T09:00:00-04:00",
            "1997-09-03T09:00:00-04:00",
            "1997-09-04T09:00:00-04:00",
            "1997-09-05T09:00:00-04:00",
            "1997-09-06T09:00:00-04:00",
            "1997-09-07T09:00:00-04:00",
            "1997-09-08T09:00:00-04:00",
            "1997-09-09T09:00:00-04:00",
            "1997-09-10T09:00:00-04:00",
            "1997-09-11T09:00:00-04:00",
        ]
    }

    add_rrule_test! {
        recur_rule_iter_daily_until_dec_24, "1997-10-23T09:00:00-04:00";
        pattern "FREQ=DAILY;UNTIL=19971224T090000" => [
            "1997-10-23T09:00:00-04:00",
            "1997-10-24T09:00:00-04:00",
            "1997-10-25T09:00:00-04:00",
            "1997-10-26T09:00:00-05:00",
            ..,
            "1997-12-22T09:00:00-05:00",
            "1997-12-23T09:00:00-05:00",
            "1997-12-24T09:00:00-05:00",
        ]
    }

    add_rrule_test! {
        recur_rule_iter_every_other_day_forever, "1997-09-02T09:00:00-04:00";
        infinite "FREQ=DAILY;INTERVAL=2" => &[
            "1997-09-02T09:00:00-04:00",
            "1997-09-04T09:00:00-04:00",
            "1997-09-06T09:00:00-04:00",
            "1997-09-08T09:00:00-04:00",
        ]
    }

    add_rrule_test! {
        recur_rule_iter_every_other_day_forever_2, "1997-09-02T09:00:00-04:00";
        pattern "FREQ=DAILY;INTERVAL=2" => [
            "1997-09-02T09:00:00-04:00",
            "1997-09-04T09:00:00-04:00",
            "1997-09-06T09:00:00-04:00",
            "1997-09-08T09:00:00-04:00",
            ..
        ]
    }

    add_rrule_test! {
        recur_rule_until_utc, "1967-04-30T02:00:00";
        finite_naive "FREQ=YEARLY;BYMONTH=4;BYDAY=-1SU;UNTIL=19730429T070000Z" => &[
            "1967-04-30T02:00:00",
            "1968-04-28T02:00:00",
            "1969-04-27T02:00:00",
            "1970-04-26T02:00:00",
            "1971-04-25T02:00:00",
            "1972-04-30T02:00:00",
            "1973-04-29T02:00:00",
        ]
    }

    add_rrule_test! {
        recur_rule_fortnightly, "2022-07-26T10:00:00-04:00";
        infinite "FREQ=WEEKLY;WKST=SU;INTERVAL=2;BYDAY=TU" => &[
            "2022-07-26T10:00:00-04:00",
            "2022-08-09T10:00:00-04:00",
            "2022-08-23T10:00:00-04:00",
        ]
    }

    add_rrule_test! {
        recur_rule_monthly, "2022-09-01T15:00:00-04:00";
        infinite "FREQ=MONTHLY;BYMONTHDAY=1" => &[
            "2022-09-01T15:00:00-04:00",
            "2022-10-01T15:00:00-04:00",
            "2022-11-01T15:00:00-04:00",
        ]
    }
}
