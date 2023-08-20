use std::marker::PhantomData;
use std::ops::Sub;
use std::time::{Duration, Instant};

pub trait Reporter<Message, ReportFunc: Fn(Message, bool) -> Result, Result> {
    fn report(&mut self, message: Message) -> Result {
        self.maybe_report(message, self.is_report_due())
    }

    fn is_report_due(&self) -> bool;

    fn force_report(&mut self, message: Message) -> Result {
        self.maybe_report(message, true)
    }

    fn maybe_report(&mut self, message: Message, is_due: bool) -> Result;
}

pub struct SimpleReporter<Message, ReportFunc: Fn(Message, bool) -> Result, Result> {
    pub last_report: Instant,
    pub min_interval: Duration,
    report_function: ReportFunc,
    message: PhantomData<Message>,
    result: PhantomData<Result>,
}

impl<Message, ReportFunc: Fn(Message, bool) -> Result, Result>
    SimpleReporter<Message, ReportFunc, Result>
{
    pub fn new(min_interval: Duration, report_function: ReportFunc) -> Self {
        Self {
            last_report: Instant::now(),
            min_interval,
            report_function,
            message: Default::default(),
            result: Default::default(),
        }
    }

    pub fn into_report_function(self) -> ReportFunc {
        self.report_function
    }
}

impl<Message, ReportFunc: Fn(Message, bool) -> Result, Result> Reporter<Message, ReportFunc, Result>
    for SimpleReporter<Message, ReportFunc, Result>
{
    fn is_report_due(&self) -> bool {
        self.last_report.elapsed() > self.min_interval
    }

    fn maybe_report(&mut self, message: Message, is_due: bool) -> Result {
        self.last_report = Instant::now();
        (self.report_function)(message, is_due)
    }
}

pub struct CountingReporter<
    Count: PartialOrd + Sub<Output = Count> + Clone,
    Message,
    ReportFunc: Fn(Message, bool) -> Result,
    Result,
> {
    pub last_report: Instant,
    pub min_interval: Duration,
    pub max_interval: Duration,
    pub last_count: Count,
    pub current_count: Count,
    pub min_count: Count,
    pub max_count: Count,
    report_function: ReportFunc,
    message: PhantomData<Message>,
    result: PhantomData<Result>,
}

impl<
        Count: PartialOrd + Sub<Output = Count> + Clone,
        Message,
        ReportFunc: Fn(Message, bool) -> Result,
        Result,
    > CountingReporter<Count, Message, ReportFunc, Result>
{
    pub fn new(
        min_interval: Duration,
        max_interval: Duration,
        current_count: Count,
        min_count: Count,
        max_count: Count,
        report_function: ReportFunc,
    ) -> Self {
        Self {
            last_report: Instant::now(),
            min_interval,
            max_interval,
            last_count: current_count.clone(),
            current_count,
            min_count,
            max_count,
            report_function,
            message: Default::default(),
            result: Default::default(),
        }
    }

    pub fn into_report_function(self) -> ReportFunc {
        self.report_function
    }
}
impl<
        Count: PartialOrd + Sub<Output = Count> + Clone + num_traits::Zero + num_traits::One,
        Message,
        ReportFunc: Fn(Message, bool) -> Result,
        Result,
    > CountingReporter<Count, Message, ReportFunc, Result>
{
    pub fn new_with_defaults(
        max_interval: Duration,
        max_count: Count,
        report_function: ReportFunc,
    ) -> Self {
        Self::new(
            Duration::from_secs(1),
            max_interval,
            Count::zero(),
            Count::one(),
            max_count,
            report_function,
        )
    }
}

impl<
        Count: PartialOrd + Sub<Output = Count> + Clone,
        Message,
        ReportFunc: Fn(Message, bool) -> Result,
        Result,
    > Reporter<Message, ReportFunc, Result>
    for CountingReporter<Count, Message, ReportFunc, Result>
{
    fn is_report_due(&self) -> bool {
        let elapsed_count = self.current_count.clone() - self.last_count.clone();
        (elapsed_count >= self.min_count && self.last_report.elapsed() >= self.min_interval)
            && (elapsed_count >= self.max_count || self.last_report.elapsed() >= self.max_interval)
    }

    fn maybe_report(&mut self, message: Message, is_due: bool) -> Result {
        self.last_report = Instant::now();
        self.last_count = self.current_count.clone();
        (self.report_function)(message, is_due)
    }
}
