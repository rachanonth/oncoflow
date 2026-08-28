export type UserRole = "pharmacist" | "admin";
export type UserType = "pharmacist" | "non_pharmacist";

export interface CurrentUser {
  id: number;
  username: string;
  displayName: string;
  role: UserRole;
  userType: UserType;
}

export interface ManagedUser extends CurrentUser {
  active: boolean;
  createdAt: string | null;
  updatedAt: string | null;
}

export interface AuthState {
  needsBootstrap: boolean;
  authenticated: boolean;
  currentUser: CurrentUser | null;
}

export interface BootstrapUserInput {
  username: string;
  displayName: string;
  password: string;
}

export interface LoginInput {
  username: string;
  password: string;
}

export interface ChangePasswordInput {
  currentPassword: string;
  newPassword: string;
}

export interface CreateUserInput {
  username: string;
  displayName: string;
  password: string;
  userType: UserType;
}

export interface UpdateUserInput {
  username: string;
  displayName: string;
  userType: UserType;
  role: UserRole;
  active: boolean;
}
