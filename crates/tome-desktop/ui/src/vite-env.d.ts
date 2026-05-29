/// <reference types="vite/client" />

// CSS Modules — Vite-native typing. Class-name keys are strings.
declare module "*.module.css" {
  const classes: { readonly [key: string]: string };
  export default classes;
}
