use wl_nl80211::{Nl80211IfaceCombLimitAttribute, Nl80211InterfaceType};

use super::{Role, one_limit};

#[test]
fn how_many_the_kernel_allows_is_not_thrown_away() {
    let read = one_limit(&[
        Nl80211IfaceCombLimitAttribute::Max(2),
        Nl80211IfaceCombLimitAttribute::Iftypes(vec![
            Nl80211InterfaceType::Station,
            Nl80211InterfaceType::Ap,
        ]),
    ]);

    assert_eq!(read.at_most, 2);
    assert_eq!(read.allows, vec![Role::OnARouter, Role::Hosting]);
}

#[test]
fn a_limit_that_names_no_number_holds_one() {
    let read = one_limit(&[Nl80211IfaceCombLimitAttribute::Iftypes(vec![
        Nl80211InterfaceType::Ap,
    ])]);

    assert_eq!(read.at_most, 1);
}
