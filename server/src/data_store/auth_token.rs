use crate::data_store::{EventId, StoreError};
use crate::CliAuthTokenKey;

pub struct EnumMemberNotExistingError;

pub struct AuthToken {
    event_id: i32,
    roles: Vec<AccessRole>,
}

impl AuthToken {
    pub fn create_for_session(
        event_id: i32,
        roles: Vec<AccessRole>,
        key: &super::AuthTokenKey,
    ) -> Self {
        AuthToken { event_id, roles }
    }

    pub fn create_for_cli(event_id: i32, key: &crate::CliAuthTokenKey) -> Self {
        let mut roles = vec![AccessRole::Admin];
        AuthToken { event_id, roles }
    }

    pub fn check_privilege(
        &self,
        event_id: EventId,
        privilege: Privilege,
    ) -> Result<(), StoreError> {
        if event_id == self.event_id
            && privilege
                .required_roles()
                .iter()
                .any(|role| self.roles.contains(role))
        {
            Ok(())
        } else {
            Err(StoreError::PermissionDenied)
        }
    }

    pub fn list_api_privileges(&self) -> Vec<kueaplan_api_types::Authorization> {
        self.roles
            .iter()
            .map(|role| kueaplan_api_types::Authorization {
                role: (*role).into(),
            })
            .collect()
    }

    pub fn has_privilege(&self, privilege_level: AccessRole) -> bool {
        self.roles.contains(&privilege_level)
    }
}

pub struct GlobalAuthToken {
    roles: Vec<AccessRole>,
}

impl GlobalAuthToken {
    pub(crate) fn check_privilege(&self, privilege: Privilege) -> Result<(), StoreError> {
        if privilege
            .required_roles()
            .iter()
            .any(|role| self.roles.contains(role))
        {
            Ok(())
        } else {
            Err(StoreError::PermissionDenied)
        }
    }

    pub fn get_global_cli_authorization(_token: &CliAuthTokenKey) -> Self {
        let mut roles = vec![AccessRole::Admin];
        GlobalAuthToken { roles }
    }
}

/// Possible roles, a single user can have with respect to a certain event
#[derive(Eq, PartialEq, Ord, PartialOrd, Clone, Copy)]
#[repr(i32)]
pub enum AccessRole {
    User = 1,
    Orga = 2,
    Admin = 3,
}

impl TryFrom<i32> for AccessRole {
    type Error = EnumMemberNotExistingError;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            1 => Ok(AccessRole::User),
            2 => Ok(AccessRole::Orga),
            3 => Ok(AccessRole::Admin),
            _ => Err(EnumMemberNotExistingError {}),
        }
    }
}

impl From<AccessRole> for kueaplan_api_types::AuthorizationRole {
    fn from(value: AccessRole) -> Self {
        match value {
            AccessRole::User => kueaplan_api_types::AuthorizationRole::Participant,
            AccessRole::Orga => kueaplan_api_types::AuthorizationRole::Orga,
            AccessRole::Admin => unimplemented!(),
        }
    }
}

pub enum Privilege {
    ShowKueaPlan,
    ManageEntries,
    ManageCategories,
    ManageRooms,
    EditEventDetails,
    ManagePassphrases,
    CreateEvents,
}

impl Privilege {
    pub fn required_roles(&self) -> &'static [AccessRole] {
        match self {
            Privilege::ShowKueaPlan => &[AccessRole::User, AccessRole::Orga, AccessRole::Admin],
            Privilege::ManageEntries => &[AccessRole::Orga, AccessRole::Admin],
            Privilege::ManageCategories => &[AccessRole::Orga, AccessRole::Admin],
            Privilege::ManageRooms => &[AccessRole::Orga, AccessRole::Admin],
            Privilege::EditEventDetails => &[AccessRole::Orga, AccessRole::Admin],
            Privilege::ManagePassphrases => &[AccessRole::Admin],
            Privilege::CreateEvents => &[AccessRole::Admin],
        }
    }
}
