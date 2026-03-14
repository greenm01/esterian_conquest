pub fn yearly_tax_revenue(present_production: u16, empire_tax_rate: u8) -> u32 {
    (u32::from(present_production) * u32::from(empire_tax_rate)) / 100
}

pub fn yearly_growth_delta(
    present_production: u16,
    potential_production: u16,
    empire_tax_rate: u8,
    has_friendly_starbase: bool,
) -> u16 {
    if present_production >= potential_production {
        return 0;
    }

    let gap = potential_production - present_production;
    let tax_headroom = 100u16.saturating_sub(u16::from(empire_tax_rate.min(95)));
    let mut growth = ((u32::from(gap) * u32::from(tax_headroom)) + 399) / 400;
    if has_friendly_starbase {
        growth += growth.div_ceil(2);
    }
    growth.max(1).min(u32::from(gap)) as u16
}

pub fn yearly_high_tax_penalty(
    present_production: u16,
    empire_tax_rate: u8,
    has_friendly_starbase: bool,
) -> u16 {
    if present_production == 0 {
        return 0;
    }

    let safe_tax = if has_friendly_starbase { 70 } else { 65 };
    if empire_tax_rate <= safe_tax {
        return 0;
    }

    let overtax = u16::from(empire_tax_rate - safe_tax);
    let penalty = ((u32::from(present_production) * u32::from(overtax)) + 499) / 500;
    penalty.max(1).min(u32::from(present_production)) as u16
}

pub fn build_capacity(present_production: u16, has_friendly_starbase: bool) -> u16 {
    if has_friendly_starbase {
        present_production.saturating_mul(5)
    } else {
        present_production
    }
}
