use time::Date as NaiveDate;
fn ymd(y: i32, m: u8, d: u8) -> Option<NaiveDate> {
    NaiveDate::from_calendar_date(y, time::Month::try_from(m).ok()?, d).ok()
}

use actual_core::cents::Cents;
use actual_core::schedules::{
    generate_upcoming_transactions, Frequency, Schedule, ScheduleConfig, TimeUnit,
};

fn dollars(d: i64) -> Cents {
    Cents::from_cents(d * 100)
}

#[test]
fn test_schedule_daily_and_weekly_recurrence() {
    let start = ymd(2026, 11, 1).unwrap();

    // 1. Daily
    let daily_config = ScheduleConfig::new(start, Frequency::Daily);
    let next_day = daily_config.next_occurrence(start).unwrap();
    assert_eq!(next_day, ymd(2026, 11, 2).unwrap());

    // 2. Weekly
    let weekly_config = ScheduleConfig::new(start, Frequency::Weekly);
    let next_week = weekly_config.next_occurrence(start).unwrap();
    assert_eq!(next_week, ymd(2026, 11, 8).unwrap());

    // 3. Biweekly (every 2 weeks)
    let biweekly_config = ScheduleConfig::new(start, Frequency::Biweekly);
    let next_biweekly = biweekly_config.next_occurrence(start).unwrap();
    assert_eq!(next_biweekly, ymd(2026, 11, 15).unwrap());
}

#[test]
fn test_schedule_monthly_with_clamping() {
    // 31st of January: February should clamp to 28 (2026 is not a leap year)
    let jan_31 = ymd(2026, 1, 31).unwrap();
    let mut config = ScheduleConfig::new(jan_31, Frequency::Monthly);
    config.day_of_month = Some(31);

    // Feb occurrence
    let feb_occurrence = config.next_occurrence(jan_31).unwrap();
    assert_eq!(feb_occurrence, ymd(2026, 2, 28).unwrap());

    // Mar occurrence returns back to 31
    let mar_occurrence = config.next_occurrence(feb_occurrence).unwrap();
    assert_eq!(mar_occurrence, ymd(2026, 3, 31).unwrap());

    // Apr occurrence clamps to 30
    let apr_occurrence = config.next_occurrence(mar_occurrence).unwrap();
    assert_eq!(apr_occurrence, ymd(2026, 4, 30).unwrap());
}

#[test]
fn test_schedule_custom_interval_and_limits() {
    let start = ymd(2026, 1, 1).unwrap();

    // Every 3 months, maximum 3 occurrences
    let mut config = ScheduleConfig::new(
        start,
        Frequency::Custom {
            interval: 3,
            unit: TimeUnit::Months,
        },
    );
    config.end_after_occurrences = Some(3);

    let occ_1 = start; // Jan 1 (occurrence 1)
    let occ_2 = config.next_occurrence(occ_1).unwrap(); // Apr 1 (occurrence 2)
    assert_eq!(occ_2, ymd(2026, 4, 1).unwrap());

    let occ_3 = config.next_occurrence(occ_2).unwrap(); // Jul 1 (occurrence 3)
    assert_eq!(occ_3, ymd(2026, 7, 1).unwrap());

    // After 3 occurrences, no more occurrences
    let occ_4 = config.next_occurrence(occ_3);
    assert_eq!(occ_4, None);
}

#[test]
fn test_generate_upcoming_transactions() {
    let start = ymd(2026, 11, 1).unwrap();
    let config = ScheduleConfig::new(start, Frequency::Weekly);

    let schedule = Schedule::new("sched_gym", "acct_checking", dollars(-50), config)
        .with_name("Gym Membership")
        .with_payee("payee_gym")
        .with_category("cat_fitness");

    let from = ymd(2026, 11, 1).unwrap();
    let until = ymd(2026, 11, 30).unwrap();

    let upcoming = generate_upcoming_transactions(&schedule, from, until);

    // In November 2026: 1st, 8th, 15th, 22nd, 29th (5 occurrences)
    assert_eq!(upcoming.len(), 5);
    assert_eq!(upcoming[0].date, ymd(2026, 11, 1).unwrap());
    assert_eq!(upcoming[1].date, ymd(2026, 11, 8).unwrap());
    assert_eq!(upcoming[2].date, ymd(2026, 11, 15).unwrap());
    assert_eq!(upcoming[3].date, ymd(2026, 11, 22).unwrap());
    assert_eq!(upcoming[4].date, ymd(2026, 11, 29).unwrap());

    for tx in &upcoming {
        assert_eq!(tx.amount, dollars(-50));
        assert_eq!(tx.schedule_id.as_deref(), Some("sched_gym"));
        assert_eq!(tx.payee_id.as_deref(), Some("payee_gym"));
        assert_eq!(tx.category_id.as_deref(), Some("cat_fitness"));
        assert!(!tx.cleared);
    }
}

#[test]
fn test_schedule_json_serde_roundtrip() {
    let start = ymd(2026, 1, 15).unwrap();
    let config = ScheduleConfig::new(start, Frequency::Monthly);

    let schedule = Schedule::new("sched_rent", "acct_bank", dollars(-1800), config)
        .with_name("Apartment Rent")
        .with_payee("payee_landlord")
        .with_category("cat_rent");

    let json = serde_json::to_string(&schedule).unwrap();
    let deserialized: Schedule = serde_json::from_str(&json).unwrap();

    assert_eq!(schedule, deserialized);
}
