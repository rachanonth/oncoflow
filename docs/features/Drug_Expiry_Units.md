# Drug expiry units

As requested on 2026-09-29, Drug data's Expiry time offers Hours and Days.
Default rate retains Minutes and Hours. A day is an elapsed 24 hours, including
fractional days; switching units preserves the duration.

The existing migration maps Access `exptime` to `drugs.expiry_time` as a time
value. Legacy clock values still represent elapsed hours, and legacy minute
values display as equivalent hours in Drug data. Existing stored values and
frozen output snapshots are not rewritten. Editing the expiry duration stores
`hr` or `day`; Rust label expiration accepts both, while retaining support for
historical clock and minute values. No MDB files are modified.

Regression checks cover the form's unit choices, conversions, and Rust day/hour
equivalence across year boundaries and leap days. This is an explicitly requested
unit extension, not a change to legacy clinical dose calculations.
