// Vitest global setup — registers @testing-library/jest-dom's custom matchers
// (`.toBeInTheDocument()`, `.toHaveTextContent()`, etc.) on Vitest's `expect`.
//
// First JS test infrastructure in the repo (Phase 26 plan 04). Reused by
// plans 26-05 and 26-07.

import "@testing-library/jest-dom";
