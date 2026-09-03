use wl_nl80211::{
    Nl80211IfaceComb, Nl80211IfaceCombAttribute, Nl80211IfaceCombLimitAttribute,
    Nl80211InterfaceType,
};

use super::{Combination, Limit, Role};

pub fn heard_from_the_kernel(combinations: &[Nl80211IfaceComb]) -> Vec<Combination> {
    combinations.iter().map(one).collect()
}

fn one(combination: &Nl80211IfaceComb) -> Combination {
    let mut read = Combination {
        each_limit: Vec::new(),
        interfaces_at_once: 1,
        channels_at_once: 1,
    };
    for attribute in &combination.attributes {
        match attribute {
            Nl80211IfaceCombAttribute::Limits(limits) => {
                read.each_limit = limits
                    .iter()
                    .map(|limit| one_limit(&limit.attributes))
                    .collect();
            }
            Nl80211IfaceCombAttribute::Maxnum(most) => read.interfaces_at_once = *most,
            Nl80211IfaceCombAttribute::NumChannels(many) => read.channels_at_once = *many,
            _ => {}
        }
    }
    read
}

fn one_limit(attributes: &[Nl80211IfaceCombLimitAttribute]) -> Limit {
    let mut read = Limit {
        allows: Vec::new(),
        at_most: 1,
    };
    for attribute in attributes {
        match attribute {
            Nl80211IfaceCombLimitAttribute::Iftypes(types) => {
                read.allows = types.iter().map(role).collect();
            }
            Nl80211IfaceCombLimitAttribute::Max(most) => read.at_most = *most,
            _ => {}
        }
    }
    read
}

fn role(kind: &Nl80211InterfaceType) -> Role {
    match kind {
        Nl80211InterfaceType::Ap => Role::Hosting,
        Nl80211InterfaceType::Station => Role::OnARouter,
        _ => Role::Something,
    }
}

#[cfg(test)]
mod tests;
