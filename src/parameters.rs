use crate::parser;

/// The valid parameters on properties
#[derive(Debug, Clone)]
pub enum Parameter {
    /// Purpose: To specify an alternate text representation for the property value.
    ///
    /// Description:  This parameter specifies a URI that points to an alternate
    /// representation for a textual property value.  A property specifying thi
    /// s parameter MUST also include a value that reflects the default
    /// representation of the text value.  The URI parameter value MUST be
    /// specified in a quoted-string.
    ///
    /// *Note: While there is no restriction imposed on the URI schemes allowed
    /// for this parameter, Content Identifier (CID) RFC2392, HTTP RFC2616, and
    /// HTTPS RFC2818 are the URI schemes most commonly used by current
    /// implementations.*
    AltRep { uri: String },

    /// Purpose: To specify the common name to be associated with the calendar
    /// user specified by the property.
    ///
    /// Description:  This parameter can be specified on properties with a
    /// CAL-ADDRESS value type.  The parameter specifies the common name to be
    /// associated with the calendar user specified by the property. The
    /// parameter value is text.  The parameter value can be used for display
    /// text to be associated with the calendar address specified by the
    /// property.
    CN(String),

    /// Purpose: To identify the type of calendar user specified by the property.
    ///
    /// Description:  This parameter can be specified on properties with a
    /// CAL-ADDRESS value type.  The parameter identifies the type of calendar
    /// user specified by the property.  If not specified on a property that
    /// allows this parameter, the default is INDIVIDUAL. Applications MUST treat
    /// x-name and iana-token values they don't recognize the same way as they
    /// would the UNKNOWN value.
    UserType(String),

    /// Purpose: To specify the calendar users that have delegated their
    /// participation to the calendar user specified by the property.
    ///
    /// Description: This parameter can be specified on properties with a
    /// CAL-ADDRESS value type.  This parameter specifies those calendar users
    /// that have delegated their participation in a group-scheduled event or
    /// to-do to the calendar user specified by the property. The individual
    /// calendar address parameter values MUST each be specified in a
    /// quoted-string.
    DelegatedFrom(Vec<String>),

    /// Purpose: To specify the calendar users to whom the calendar user
    /// specified by the property has delegated participation.
    ///
    /// Description: This parameter can be specified on properties with a
    /// CAL-ADDRESS value type.  This parameter specifies those calendar
    /// users whom have been delegated participation in a group-scheduled
    /// event or to-do by the calendar user specified by the property.
    /// The individual calendar address parameter values MUST each be
    /// specified in a quoted-string.
    DelegatedTo(Vec<String>),

    /// Purpose: To specify reference to a directory entry associated with
    /// the calendar user specified by the property.
    ///
    /// Description: This parameter can be specified on properties with a
    /// CAL-ADDRESS value type.  The parameter specifies a reference to
    /// the directory entry associated with the calendar user specified by
    /// the property.  The parameter value is a URI.  The URI parameter
    /// value MUST be specified in a quoted-string.
    ///
    /// *Note: While there is no restriction imposed on the URI schemes
    /// allowed for this parameter, CID RFC2392, DATA RFC2397, FILE
    /// RFC1738, FTP RFC1738, HTTP RFC2616, HTTPS RFC2818, LDAP
    /// RFC4516, and MID RFC2392]are the URI schemes most commonly
    /// used by current implementations.*
    Dir { uri: String },

    /// Purpose: To specify an alternate inline encoding for the property value.
    ///
    /// Description: This property parameter identifies the inline encoding used
    /// in a property value.  The default encoding is "8BIT", corresponding to a
    /// property value consisting of text.  The "BASE64" encoding type
    /// corresponds to a property value encoded using the "BASE64" encoding
    /// defined in RFC2045.
    ///
    /// If the value type parameter is ";VALUE=BINARY", then the inline encoding
    /// parameter MUST be specified with the value ";ENCODING=BASE64".
    Encoding(String),

    /// Purpose:  To specify the content type of a referenced object.
    ///
    /// Description:  This parameter can be specified on properties that are
    /// used to reference an object.  The parameter specifies the media type
    /// RFC4288 of the referenced object.  For example, on the "ATTACH"
    /// property, an FTP type URI value does not, by itself, necessarily convey
    /// the type of content associated with the resource.  The parameter value
    /// MUST be the text for either an IANA-registered media type or a
    /// non-standard media type.
    FormatType(String),

    /// Purpose:  To specify the free or busy time type.
    ///
    /// Description: This parameter specifies the free or busy time type. The
    /// value FREE indicates that the time interval is free for scheduling.  The
    /// value BUSY indicates that the time interval is busy because one or more
    /// events have been scheduled for that interval.  The value BUSY-UNAVAILABLE
    /// indicates that the time interval is busy and that the interval can not be
    /// scheduled.  The value BUSY-TENTATIVE indicates that the time interval is
    /// busy because one or more events have been tentatively scheduled for that
    /// interval.  If not specified on a property that allows this parameter, the
    /// default is BUSY.  Applications MUST treat x-name and iana-token values
    /// they don't recognize the same way as they would the BUSY value.
    FreeBusy(String),

    /// Purpose: To specify the language for text values in a property or
    /// property parameter.
    ///
    /// Description:  This parameter identifies the language of the text in the
    /// property value and of all property parameter values of the property. The
    /// value of the "LANGUAGE" property parameter is that defined in RFC5646.
    ///
    /// For transport in a MIME entity, the Content-Language header field can be
    /// used to set the default language for the entire body part. Otherwise, no
    /// default language is assumed
    Language(String),

    /// Purpose: To specify the group or list membership of the
    /// calendar user specified by the property.
    ///
    /// Description: This parameter can be specified on properties with a
    /// CAL-ADDRESS value type.  The parameter identifies the groups or list
    /// membership for the calendar user specified by the property. The parameter
    /// value is either a single calendar address in a quoted-string or a
    /// COMMA-separated list of calendar addresses, each in a quoted-string.  The
    /// individual calendar address parameter values MUST each be specified in a
    /// quoted-string.
    Member(Vec<String>),

    /// Purpose: To specify the participation status for the calendar user
    /// specified by the property.
    ///
    /// Description: This parameter can be specified on properties with a
    /// CAL-ADDRESS value type.  The parameter identifies the participation
    /// status for the calendar user specified by the property value.  The
    /// parameter values differ depending on whether they are associated with a
    /// group-scheduled "VEVENT", "VTODO", or "VJOURNAL".  The values MUST match
    /// one of the values allowed for the given calendar component.  If not
    /// specified on a property that allows this parameter, the default value is
    /// NEEDS-ACTION. Applications MUST treat x-name and iana-token values they
    /// don't recognize the same way as they would the NEEDS-ACTION value.
    ParticipationStatus(String),

    /// Purpose: To specify the effective range of recurrence instances from the
    /// instance specified by the recurrence identifier specified by the
    /// property.
    ///
    /// Description: This parameter can be specified on a property that
    /// specifies a recurrence identifier.  The parameter specifies the
    /// effective range of recurrence instances that is specified by the
    /// property.  The effective range is from the recurrence identifier
    /// specified by the property.  If this parameter is not specified on an
    /// allowed property, then the default range is the single instance
    /// specified by the recurrence identifier value of the property.  The
    /// parameter value can only be "THISANDFUTURE" to indicate a range defined
    /// by the recurrence identifier and all subsequent instances. The value
    /// "THISANDPRIOR" is deprecated by this revision of iCalendar and MUST NOT
    /// be generated by applications.
    Range(String),

    /// Purpose: To specify the relationship of the alarm trigger with respect
    /// to the start or end of the calendar component.
    ///
    /// Description: This parameter can be specified on properties that specify
    /// an alarm trigger with a "DURATION" value type.  The parameter specifies
    /// whether the alarm will trigger relative to the start or end of the
    /// calendar component.  The parameter value START will set the alarm to
    /// trigger off the start of the calendar component; the parameter value END
    /// will set the alarm to trigger off the end of the calendar component.  If
    /// the parameter is not specified on an allowable property, then the
    /// default is START.
    Related(String),

    /// Purpose: To specify the type of hierarchical relationship associated
    /// with the calendar component specified by the property.
    ///
    /// Description:  This parameter can be specified on a property that
    /// references another related calendar.  The parameter specifies the
    /// hierarchical relationship type of the calendar component referenced by
    /// the property.  The parameter value can be PARENT, to indicate that the
    /// referenced calendar component is a superior of calendar component; CHILD
    /// to indicate that the referenced calendar component is a subordinate of
    /// the calendar component; or SIBLING to indicate that the referenced
    /// calendar component is a peer of the calendar component.  If this
    /// parameter is not specified on an allowable property, the default
    /// relationship type is PARENT. Applications MUST treat x-name and
    /// iana-token values they don't recognize the same way as they would the
    /// PARENT value.
    RelationshipType(String),

    /// Purpose: To specify the participation role for the calendar user
    /// specified by the property.
    ///
    /// Description:  This parameter can be specified on properties with a
    /// CAL-ADDRESS value type.  The parameter specifies the participation role
    /// for the calendar user specified by the property in the group schedule
    /// calendar component.  If not specified on a property that allows this
    /// parameter, the default value is REQ-PARTICIPANT. Applications MUST treat
    /// x-name and iana-token values they don't recognize the same way as they
    /// would the REQ-PARTICIPANT value.
    ParticipationRole(String),

    /// Purpose:  To specify whether there is an expectation of a favor of a
    /// reply from the calendar user specified by the property value.
    ///
    /// Description:  This parameter can be specified on properties with a
    /// CAL-ADDRESS value type.  The parameter identifies the expectation of a
    /// reply from the calendar user specified by the property value. This
    /// parameter is used by the "Organizer" to request a participation status
    /// reply from an "Attendee" of a group-scheduled event or to-do.  If not
    /// specified on a property that allows this parameter, the default value is
    /// FALSE.
    RSVPExpectation(bool),

    /// Purpose: To specify the calendar user that is acting on behalf of the
    /// calendar user specified by the property.
    ///
    /// Description: This parameter can be specified on properties with a
    /// CAL-ADDRESS value type.  The parameter specifies the calendar user that
    /// is acting on behalf of the calendar user specified by the property.  The
    /// parameter value MUST be a mailto URI as defined in RFC2368.  The
    /// individual calendar address parameter values MUST each be specified in a
    /// quoted-string.
    SentBy(String),

    /// Purpose: To specify the identifier for the time zone definition for a
    /// time component in the property value.
    ///
    /// Description:  This parameter MUST be specified on the "DTSTART",
    /// "DTEND", "DUE", "EXDATE", and "RDATE" properties when either a DATE-TIME
    /// or TIME value type is specified and when the value is neither a UTC or a
    /// "floating" time.  Refer to the DATE-TIME or TIME value type definition
    /// for a description of UTC and "floating time" formats.  This property
    /// parameter specifies a text value that uniquely identifies the
    /// "VTIMEZONE" calendar component to be used when evaluating the time
    /// portion of the property.  The value of the "TZID" property parameter
    /// will be equal to the value of the "TZID" property for the matching time
    /// zone definition. An individual "VTIMEZONE" calendar component MUST be
    /// specified for each unique "TZID" parameter value specified in the
    /// iCalendar object.
    ///
    /// The parameter MUST be specified on properties with a DATE-TIME value if
    /// the DATE-TIME is not either a UTC or a "floating" time. Failure to
    /// include and follow VTIMEZONE definitions in iCalendar objects may lead
    /// to inconsistent understanding of the local time at any given location.
    ///
    /// The presence of the SOLIDUS character as a prefix, indicates that this
    /// "TZID" represents a unique ID in a globally defined time zone registry
    /// (when such registry is defined).
    ///
    /// *Note: This document does not define a naming convention for time zone
    /// identifiers.  Implementers may want to use the naming conventions
    /// defined in existing time zone specifications such as the public-domain
    /// TZ database (TZDB).  The specification of globally unique time zone
    /// identifiers is not addressed by this document and is left for future
    /// study.*
    TimeZoneID(String),

    /// Purpose:  To explicitly specify the value type format for a property
    /// value.
    ///
    /// Description:  This parameter specifies the value type and format of the
    /// property value.  The property values MUST be of a single value type. For
    /// example, a "RDATE" property cannot have a combination of DATE-TIME and
    /// TIME value types.
    ///
    /// If the property's value is the default value type, then this parameter
    /// need not be specified.  However, if the property's default value type is
    /// overridden by some other allowable value type, then this parameter MUST
    /// be specified.
    ///
    /// Applications MUST preserve the value data for x-name and iana- token
    /// values that they don't recognize without attempting to interpret or
    /// parse the value data.
    ValueDataType(String),

    /// Any parameter that wasn't recognized.
    Other { name: String, values: Vec<String> },
}

impl From<parser::Parameter> for Parameter {
    fn from(p: parser::Parameter) -> Self {
        // Note: we have already asserted that the values have at least one
        // entry, hence the `.except(..)`.
        match &p.name.to_ascii_uppercase() as &str {
            "ALTREP" => Parameter::AltRep {
                uri: p.values.into_iter().last().expect("values"),
            },
            "CN" => Parameter::CN(p.values.into_iter().last().expect("values")),
            "CUTYPE" => Parameter::UserType(p.values.into_iter().last().expect("values")),
            "DELEGATED-FROM" => Parameter::DelegatedFrom(p.values),
            "DELEGATED-TO" => Parameter::DelegatedTo(p.values),
            "DIR" => Parameter::Dir {
                uri: p.values.into_iter().last().expect("values"),
            },
            "ENCODING" => Parameter::Encoding(p.values.into_iter().last().expect("values")),
            "FMTTYPE" => Parameter::FormatType(p.values.into_iter().last().expect("values")),
            "FBTYPE" => Parameter::FreeBusy(p.values.into_iter().last().expect("values")),
            "LANGUAGE" => Parameter::Language(p.values.into_iter().last().expect("values")),
            "MEMBER" => Parameter::Member(p.values),
            "PARTSTAT" => {
                Parameter::ParticipationStatus(p.values.into_iter().last().expect("values"))
            }
            "RANGE" => Parameter::Range(p.values.into_iter().last().expect("values")),
            "RELATED" => Parameter::Related(p.values.into_iter().last().expect("values")),
            "RELTYPE" => Parameter::RelationshipType(p.values.into_iter().last().expect("values")),
            "ROLE" => Parameter::ParticipationRole(p.values.into_iter().last().expect("values")),
            "RSVP" => Parameter::RSVPExpectation(
                p.values
                    .into_iter()
                    .last()
                    .expect("values")
                    .to_ascii_uppercase()
                    == "TRUE",
            ),
            "SENT-BY" => Parameter::SentBy(p.values.into_iter().last().expect("values")),
            "TZID" => Parameter::TimeZoneID(p.values.into_iter().last().expect("values")),
            "VALUE" => Parameter::ValueDataType(p.values.into_iter().last().expect("values")),

            _ => Parameter::Other {
                name: p.name.to_ascii_uppercase(),
                values: p.values,
            },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ParameterSet {
    parameters: Vec<Parameter>,
}

impl<I> From<I> for ParameterSet
where
    I: IntoIterator<Item = parser::Parameter>,
{
    fn from(iter: I) -> Self {
        ParameterSet {
            parameters: iter.into_iter().map(Parameter::from).collect(),
        }
    }
}

impl ParameterSet {
    pub fn parameters(&self) -> &[Parameter] {
        &self.parameters
    }

    pub fn get_value_data_type(&self) -> Option<&str> {
        for param in &self.parameters {
            if let Parameter::ValueDataType(data_type) = param {
                return Some(data_type);
            }
        }

        None
    }

    pub fn get_encoding(&self) -> Option<&str> {
        for param in &self.parameters {
            if let Parameter::Encoding(encoding) = param {
                return Some(encoding);
            }
        }

        None
    }
    pub fn get_tzid(&self) -> Option<&str> {
        for param in &self.parameters {
            if let Parameter::TimeZoneID(tzid) = param {
                return Some(tzid);
            }
        }

        None
    }
}
