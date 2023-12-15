use clocksource::coarse::UnixInstant;
use std::time::SystemTime;

fn to_unix_ns(t: SystemTime) -> u64 {
    t.duration_since(SystemTime::UNIX_EPOCH).unwrap().as_nanos() as u64
}

#[test]
fn realtime_coarse() {
    // the realtime clock may jump backward, so we may need to try a few times
    for _ in 0..5 {
        let t0 = SystemTime::now();
        let t1 = UnixInstant::now();
        let t2 = SystemTime::now();
        let t3 = UnixInstant::now();
        let t4 = SystemTime::now();

        // convert our times into durations since the unix epoch
        let ut0 = to_unix_ns(t0);
        let ut1 = t1.duration_since(UnixInstant::EPOCH).as_secs();
        let ut2 = to_unix_ns(t2);
        let ut3 = t3.duration_since(UnixInstant::EPOCH).as_secs();
        let ut4 = to_unix_ns(t4);

        // check that the clock has moved forward and not backward
        if t0 < t2 && t2 < t4 {
            let ut0 = (ut0 / 1_000_000_000) as u32;
            let ut2 = (ut2 / 1_000_000_000) as u32;
            let ut4 = (ut4 / 1_000_000_000) as u32;

            assert!(ut0 <= ut1, "ut0: {ut0} ut1: {ut1}");
            assert!(ut1 <= ut2, "ut1: {ut1} ut2: {ut2}");
            assert!(ut2 <= ut3, "ut2: {ut2} ut3: {ut3}");
            assert!(ut3 <= ut4, "ut3: {ut3} ut4: {ut4}");
        }
    }
}
