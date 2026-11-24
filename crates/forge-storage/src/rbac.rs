use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Role {
    Admin,
    Member,
    Viewer,
    Runner,
}

impl Role {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Admin => "admin",
            Self::Member => "member",
            Self::Viewer => "viewer",
            Self::Runner => "runner",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "admin" => Some(Self::Admin),
            "member" => Some(Self::Member),
            "viewer" => Some(Self::Viewer),
            "runner" => Some(Self::Runner),
            _ => None,
        }
    }

    pub fn can_manage_secrets(self) -> bool {
        matches!(self, Self::Admin | Self::Member)
    }

    pub fn can_register_runner(self) -> bool {
        matches!(self, Self::Admin)
    }

    pub fn can_read_org(self) -> bool {
        !matches!(self, Self::Runner)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Permission {
    ReadOrganization,
    ManageRunners,
    ManageSecrets,
    ManageProjects,
    ManageBilling,
    AdminOnly,
}

impl Permission {
    pub fn allowed(role: Role) -> bool {
        Self::allowed_for(role, Self::ReadOrganization)
    }

    pub fn allowed_for(role: Role, permission: Permission) -> bool {
        match permission {
            Permission::ReadOrganization => role.can_read_org(),
            Permission::ManageRunners => role.can_register_runner(),
            Permission::ManageSecrets => role.can_manage_secrets(),
            Permission::ManageProjects => matches!(role, Role::Admin | Role::Member),
            Permission::ManageBilling => matches!(role, Role::Admin),
            Permission::AdminOnly => matches!(role, Role::Admin),
        }
    }
}
