pub trait IsPlural: PartialEq {
    fn is_singular(&self) -> bool;
    fn is_plural(&self) -> bool {
        !self.is_singular()
    }
}

macro_rules ! impl_is_singular {
    ($($ty:ty: [$pos:literal$(, $neg:literal)?]),*) => {
        $(
            impl IsPlural for $ty {
                fn is_singular(&self) -> bool {
                    if self == &$pos {
                        true
                    }
                    $(
                        else if self == &$neg {
                            true
                        }
                    )?
                     else {
                        false
                     }
                }
            }
        )*
    }
}

impl_is_singular!(
    u8: [1],
    u16: [1],
    u32: [1],
    u64: [1],
    u128: [1],
    i8: [1, -1],
    i16: [1, -1],
    i32: [1, -1],
    i64: [1, -1],
    i128: [1, -1],
    f32: [1.0, -1.0],
    f64: [1.0, -1.0]
);

macro_rules ! unit {
    {
        $($name:ident = [$value:literal, $short:expr, $singular:literal, $plural:literal],)*
    } => {
        #[derive(Debug, Clone, Copy, PartialEq)]
        #[repr(u8)]
        pub enum Unit {
            $($name = $value,)*
            Unknown(u8),
        }

        impl Unit {
            fn short_display(&self) -> Option<&str> {
                match self {
                    $(Self::$name => $short,)*
                    Self::Unknown(_) => None,
                }
            }

            fn plural_display(&self) -> &str {
                match self {
                    $(Self::$name => $plural,)*
                    Self::Unknown(_) => "Unknown",
                }
            }

            fn singular_display(&self) -> &str {
                match self {
                    $(Self::$name => $singular,)*
                    Self::Unknown(_) => "Unknown",
                }
            }

            fn display_str(&self, short: bool, plural: bool) -> &str
            {
                let short_value = if short {
                    self.short_display()
                } else {
                    None
                };

                if let Some(short_value) = short_value {
                    short_value
                } else {
                    if plural {
                        self.plural_display()
                    } else {
                        self.singular_display()
                    }
                }
            }

            pub fn display<T>(&self, short: bool, value: T) -> String
                where T: IsPlural + core::fmt::Display,
            {
                format!("{:.2} {}", value, self.display_str(short, value.is_plural()))
            }
        }

        impl From<u8> for Unit {
            fn from(value: u8) -> Self {
                match value {
                    $($value => Self::$name,)*
                    _ => Self::Unknown(value),
                }
            }
        }

        impl From<Unit> for u8 {
            fn from(value: Unit) -> Self {
                match value {
                    $(Unit::$name => $value,)*
                    Unit::Unknown(v) => v,
                }
            }
        }

    }
}

unit! {
    Unspecified = [0, None, "Unspecified", "Unspecified"],
    DegreesCelcius = [1, Some("°C"), "Degree Celcius", "Degrees Celcius"],
    DegreesFahrenheit = [2, Some("°F"), "Degree Fahrenheit", "Degrees Fahrenheit"],
    DegreesKelvin = [3, Some("°K"), "Degree Kelvin", "Degrees Kelvin"],
    Volt = [4, Some("V"), "Volt", "Volts"],
    Amp = [5, Some("A"), "Ampere", "Amperes"],
    Watt = [6, Some("W"), "Watt", "Watts"],
    Joule = [7, Some("J"), "Joule", "Joules"],
    Coulomb = [8, Some("C"), "Coulomb", "Coulombs"],
    VoltAmpere = [9, Some("VA"), "Volt-Ampere", "Volt-Amperes"],
    Nit = [10, None, "Nit", "Nits"],
    Lumen = [11, Some("lm"), "Lumen", "Lumens"],
    Lux = [12, Some("lx"), "Lux", "Lux"],
    Candela = [13, Some("cd"), "Candela", "Candelas"],
    KiloPascal = [14, Some("kPa"), "Kilopascal", "Kilopascal"],
    PoundsPerSquareInch = [15, Some("psi"), "Pound per square inch", "Pounds per square inch"],
    Newton = [16, Some("N"), "Newton", "Newton"],
    CubicFeetPerMinute = [17, Some("cfm"), "Cubic Foot per Minute", "Cubic Feet per minute"],
    RevolutionsPerMinute = [18, Some("rpm"), "Revolution per Minute", "Revolutions per Minute"],
    Hertz = [19, Some("hz"), "Hertz", "Hertz"],
    Microsecond = [20, Some("µs"), "Microsecond", "Microseconds"],
    Millisecond = [21, Some("ms"), "Millisecond", "Milliseconds"],
    Second = [22, Some("s"), "Second", "Seconds"],
    Minute = [23, Some("min"), "Minute", "Minutes"],
    Hour = [24, Some("h"), "Hour", "Hours"],
    Day = [25, Some("d"), "Day", "Days"],
    Week = [26, Some("w"), "Week", "Weeks"],
    Mil = [27, Some("mil"), "mil", "mils"],
    Inch = [28, Some("in"), "Inch", "Inches"],
    Foot = [29, Some("ft"), "Foot", "Feet"],
    CubicInch = [30, Some("cu in"), "Cubic Inch", "Cubic Inches"],
    CubicFoot = [31, Some("cu ft"), "Cubic Foot", "Cubic Feet"],
    Millimeter = [32, Some("mm"), "Millimeter", "Millimeters"],
    Centimeter = [33, Some("cm"), "Centimeter", "Centimeters"],
    Meter = [34, Some("m"), "Meter", "Meters"],
    CubicCentimeter = [35, Some("cu cm"), "Cubic Centimeter", "Cubic Centimeters"],
    CubicMeter = [36, Some("cu m"), "Cubic Meter", "Cubic Meters"],
    Liter = [37, Some("l"), "Liter", "Liters"],
    FluidOunce = [38, Some("fl oz"), "Fluid Ounce", "Fluid Ounces"],
    Radian = [39, Some("rad"), "Radian", "Radians"],
    Steradian = [40, Some("sr"), "Steradian", "Steradians"],
    Revolution = [41, Some("rev"), "Revolution", "Revolutions"],
    Cycle = [42, None, "Cycle", "Cycles"],
    Gravity = [43, Some("Gs (grav)"), "Gravity", "Gravities"],
    Ounce = [44, Some("oz"), "Ounce", "Ounces"],
    Pound = [45, Some("lb"), "Pound", "Pounds"],
    FootPound = [46, Some("ft lb"), "Foot Pound", "Foot Pounds"],
    OunceInch = [47, Some("oz in"), "Ounce Inch", "Ounce Inches"],
    Gauss = [48, Some("Gs"), "Gauss", "Gauss"],
    Gilbert = [49, Some("Gb"), "Gilbert", "Gilbert"],
    Henry = [50, Some("H"), "Heny", "Heny"],
    Millihenry = [51, Some("mH"), "Millihenry", "Millihenry"],
    Farad = [52, Some("F"), "Farad", "Farad"],
    Microfarad = [53, Some("µF"), "Microfarad", "Microfarad"],
    Ohm = [54, Some("Ω"), "Ohm", "Ohm"],
    Siemens = [55, Some("S"), "Siemens", "Siemens"],
    Mole = [56, Some("mol"), "Mol", "Mol"],
    Becquerel = [57, Some("Bq"), "Becquerel", "Becquerel"],
    PartsPerMillion = [58, Some("ppm"), "Part part per million", "Parts per million"],
    Decibel = [60, Some("dB"), "Decibel", "Decibels"],
    AWeightedDecibel = [61, Some("dBa"), "A weighted Decibel", "A weighted Decibels"],
    CWeightedDecibel = [62, Some("dBc"), "C weighted Decibel", "C weighted Decibels"],
    Gray = [63, Some("Gy"), "Gray", "Gray"],
    Sievert = [64, Some("Sv"), "Sievert", "Sievert"],
    ColorTemperatureDegreesKelvin = [65, Some("°K (Color)"), "Degree Kelvin (Color)", "Degrees Kelvin (Color)"],
    Bit = [66, Some("b"), "Bit", "Bits"],
    Kilobit = [67, Some("kb"), "Kilobit", "Kilobits"],
    Megabit = [68, Some("mb"), "Megabit", "Megabits"],
    Gigabit = [69, Some("Gb"), "Gigabit", "Gigabits"],
    Byte = [70, Some("B"), "Byte", "Bytes"],
    Kilobyte = [71, Some("KB"), "Kilobyte", "Kilobytes"],
    Megabyte = [72, Some("MB"), "Megabyte", "Megabytes"],
    Gigabyte = [73, Some("GB"), "Gigabyte", "Gigabytes"],
    Word = [74, None, "Word", "Words"],
    DoubleWord = [75, None, "Double word", "Double words"],
    QuadWord = [76, None, "Quad word", "Quad words"],
    CacheLine = [77, None, "Cache line", "Cache lines"],
    Hit = [78, None, "Hit", "Hits"],
    Miss = [79, None, "Miss", "Misses"],
    Retry = [80, None, "Retry", "Retries"],
    Reset = [81, None, "Reset", "Resets"],
    OverrunOrUnderflow = [82, None, "Overrun or Underflow", "Overruns or Underflows"],
    Underrun = [83, None, "Underrun", "Underruns"],
    Collision = [84, None, "Collision", "Collisions"],
    Packet = [85, None, "Packet", "Packets"],
    Message = [86, None, "Message", "Messges"],
    Character = [87, None, "Character", "Characters"],
    Error = [88, None, "Error", "Errors"],
    CorrectableError = [89, None, "Correctable Error", "Correctable Errors"],
    UncorrectableError = [90, None, "Uncorrectable Error", "Uncorrectable Errors"],
    FatalError = [91, None, "Fatal Error", "Fatal Errors"],
    Gram = [92, Some("gr"), "Gram", "Grams"],
}

#[test]
fn display_tests() {
    use Unit::*;

    assert_eq!("32 °C", DegreesCelcius.display(true, 32));
    assert_eq!("32 Degrees Celcius", DegreesCelcius.display(false, 32));
    assert_eq!("1 Degree Celcius", DegreesCelcius.display(false, 1));
    assert_eq!("-1 Nit", Nit.display(true, -1));
    assert_eq!("-1 Nit", Nit.display(false, -1));
    assert_eq!("-15 Nits", Nit.display(false, -15));
}
