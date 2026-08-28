export interface BootstrapFormValues {
  username: string;
  displayName: string;
  password: string;
  confirmPassword: string;
}

export interface PasswordFormValues {
  currentPassword: string;
  newPassword: string;
  confirmPassword: string;
}

export type FormErrors = Partial<Record<keyof BootstrapFormValues | keyof PasswordFormValues, string>>;

export function validateBootstrap(values: BootstrapFormValues): FormErrors {
  const errors: FormErrors = {};
  const username = values.username.trim();
  if (username.length < 3 || username.length > 64 || /\s/.test(username)) errors.username = "Use 3–64 characters without spaces.";
  if (!values.displayName.trim() || values.displayName.trim().length > 100) errors.displayName = "Display name is required and limited to 100 characters.";
  if (values.password.length < 12 || values.password.length > 128) errors.password = "Use a password between 12 and 128 characters.";
  else if (values.password.toLocaleLowerCase() === username.toLocaleLowerCase()) errors.password = "Password must be different from the username.";
  if (values.confirmPassword !== values.password) errors.confirmPassword = "Passwords do not match.";
  return errors;
}

export function validatePasswordChange(values: PasswordFormValues): FormErrors {
  const errors: FormErrors = {};
  if (!values.currentPassword) errors.currentPassword = "Enter the current password.";
  if (values.newPassword.length < 12 || values.newPassword.length > 128) errors.newPassword = "Use a password between 12 and 128 characters.";
  if (values.confirmPassword !== values.newPassword) errors.confirmPassword = "Passwords do not match.";
  return errors;
}
