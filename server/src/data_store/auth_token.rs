use crate::data_store::{EventId, StoreError};
use crate::CliAuthTokenKey;

pub struct EnumMemberNotExistingError;

/// Authorization token for authorizing access to the data_store for a specific event
///
/// The AuthToken is keyed to a specific event (by its event id) and holds a list of active
/// [AccessRole]s in the current context. These imply specific [Privilege]s.
///
/// This structure is our main protection against accidental unauthorized-access bugs: All
/// data_store access function require an AuthToken and check the validity of the AuthToken
/// for the required event id and privilege. An AuthToken can only be created by
/// [crate::data_store::KueaPlanStoreFacade::check_authorization], based on the authenticated
/// passphrases in a client's session, and by cli functions via [create_for_cli].
///
/// For global, not event-specific authorization, a GlobalAuthToken is required instead.
pub struct AuthToken {
    event_id: i32,
    roles: Vec<AccessRole>,
}

impl AuthToken {
    /// Create a new AuthToken for a client session, based on the access roles of the authenticated
    /// passphrases of that client typically retrieved from the client's
    /// [crate::auth_session::SessionToken].
    ///
    /// This function must only be used by implementations of
    /// [crate::data_store::KueaPlanStoreFacade::check_authorization]
    /// after checking the validity of the client's authenticated passphrase ids and their implied
    /// user roles!
    pub(super) fn create_for_session(event_id: i32, roles: Vec<AccessRole>) -> Self {
        AuthToken { event_id, roles }
    }

    /// Create a new AuthToken for a command line interface functionality.
    ///
    /// The AuthToken is created with the AccessRole::Admin.
    ///
    /// This function must only be used by command line interface functions, not in the context of
    /// the web server!
    pub fn create_for_cli(event_id: i32, key: &crate::CliAuthTokenKey) -> Self {
        let mut roles = vec![AccessRole::Admin];
        AuthToken { event_id, roles }
    }

    /// Check if the AuthToken authorizes for the given `privilege`. If not, return an appropriate
    /// PermissionDenied error.
    ///
    /// The actual authorization check is delegated to [Privilege::required_roles], by checking if
    /// any of the active in the context (i.e. roles contained in the AuthToken)
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
            Err(StoreError::PermissionDenied {
                required_privilege: privilege,
            })
        }
    }

    /// Get the list of active access roles in the API representation.
    ///
    /// This is used by the [crate::web::api::endpoints_auth::check_authorization] endpoint,
    /// allowing the client to query its current active roles.
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

/// Authorization token for authorizing access to the data_store for global (not event-specific
/// actions).
///
/// Together with [AuthToken], this structure is our main protection against accidental
/// unauthorized-access bugs: All non-event-specific data_store access function require to pass a
/// GlobalAuthToken and check its validity for the required privilege. An GlobalAuthToken can only
/// be created by cli functions via [get_global_cli_authorization].
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
            Err(StoreError::PermissionDenied {
                required_privilege: privilege,
            })
        }
    }

    pub fn get_global_cli_authorization(_token: &CliAuthTokenKey) -> Self {
        let mut roles = vec![AccessRole::Admin];
        GlobalAuthToken { roles }
    }
}

/// Possible roles, a client can authenticate for, using passphrases.
///
/// Each role qualifies for a set of [Privileges]. See [Privilege::required_roles].
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

/// Enum of available authorization privileges.
///
/// Each data_store action and web endpoint typically requires a single privilege.
#[derive(Debug)]
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
    /// Get the list of user [AccessRole]s that qualify for this privilege. Each returned role is
    /// individually sufficient for the privilege.
    ///
    /// This is function is our source of truth for authorization!
    /// It can also be used to inform the user about possible roles they would need to authenticate
    /// for, in order to unlock a specific action.
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
