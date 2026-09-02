use wl_nl80211::{
    Nl80211IfaceComb, Nl80211IfaceCombAttribute, Nl80211IfaceCombLimitAttribute,
    Nl80211InterfaceType,
};

use super::{Combination, Role};

pub fn heard_from_the_kernel(combinations: &[Nl80211IfaceComb]) -> Vec<Combination> {
    combinations.iter().map(one).collect()
}

fn one(combination: &Nl80211IfaceComb) -> Combination {
    let mut read = Combination {
        each_limit_allows: Vec::new(),
        interfaces_at_once: 1,
        channels_at_once: 1,
    };
    for attribute in &combination.attributes {
        match attribute {
            Nl80211IfaceCombAttribute::Limits(limits) => {
                read.each_limit_allows = limits.iter().map(roles_in).collect();
            }
            Nl80211IfaceCombAttribute::Maxnum(most) => read.interfaces_at_once = *most,
            Nl80211IfaceCombAttribute::NumChannels(many) => read.channels_at_once = *many,
            _ => {}
        }
    }
    read
}

fn roles_in(limit: &wl_nl80211::Nl80211IfaceCombLimit) -> Vec<Role> {
    limit
        .attributes
        .iter()
        .filter_map(|attribute| match attribute {
            Nl80211IfaceCombLimitAttribute::Iftypes(types) => Some(types.iter().map(role)),
            _ => None,
        })
        .flatten()
        .collect()
}

fn role(kind: &Nl80211InterfaceType) -> Role {
    match kind {
        Nl80211InterfaceType::Ap => Role::Hosting,
        Nl80211InterfaceType::Station => Role::OnARouter,
        _ => Role::Something,
    }
}
