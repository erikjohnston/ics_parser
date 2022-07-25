use std::convert::TryFrom;

use anyhow::Error;
use ics_parser::{components::VCalendar, parser};

#[test]
fn test_recur_id_different_tz() -> Result<(), Error> {
    let vcal_raw = r#"BEGIN:VCALENDAR
PRODID:-//Google Inc//Google Calendar 70.9054//EN
VERSION:2.0
BEGIN:VTIMEZONE
TZID:Europe/London
X-LIC-LOCATION:Europe/London
BEGIN:DAYLIGHT
TZOFFSETFROM:+0000
TZOFFSETTO:+0100
TZNAME:BST
DTSTART:19700329T010000
RRULE:FREQ=YEARLY;BYMONTH=3;BYDAY=-1SU
END:DAYLIGHT
BEGIN:STANDARD
TZOFFSETFROM:+0100
TZOFFSETTO:+0000
TZNAME:GMT
DTSTART:19701025T020000
RRULE:FREQ=YEARLY;BYMONTH=10;BYDAY=-1SU
END:STANDARD
END:VTIMEZONE
BEGIN:VEVENT
DTSTART;TZID=Europe/London:20220208T153000
DTEND;TZID=Europe/London:20220208T162000
RRULE:FREQ=WEEKLY;INTERVAL=2
DTSTAMP:20220712T145025Z
ORGANIZER;CN=Foo:mailto:foo@example.org
UID:26a0c5d5-50e8-4ae0-a2fd-80968f6db384
DESCRIPTION:
SEQUENCE:2
SUMMARY:Test
END:VEVENT
BEGIN:VEVENT
DTSTART;TZID=Europe/London:20220222T153000
DTEND;TZID=Europe/London:20220222T162000
DTSTAMP:20220712T145025Z
ORGANIZER;CN=Foo:mailto:foo@example.org
UID:26a0c5d5-50e8-4ae0-a2fd-80968f6db384
RECURRENCE-ID:20220222T153000Z
DESCRIPTION:
SEQUENCE:11
SUMMARY:Test Edit
END:VEVENT
END:VCALENDAR
"#;

    let components = parser::Component::from_str_to_stream(vcal_raw)?;
    let component = components.into_iter().next().unwrap();

    let vcalendar = VCalendar::try_from(component)?;

    let collection = vcalendar
        .events
        .get("26a0c5d5-50e8-4ae0-a2fd-80968f6db384")
        .unwrap();

    let mut summaries = Vec::new();
    for (_, vevent) in collection.recur_iter(&vcalendar)?.take(3) {
        summaries.push(vevent.summary.as_deref().unwrap());
    }

    assert_eq!(&summaries, &["Test", "Test Edit", "Test"]);

    Ok(())
}
