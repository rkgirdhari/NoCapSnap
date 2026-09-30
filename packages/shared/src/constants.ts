export const COMPANY = {
  legalName: "Hammurabi Coding Company LLC",
  founder: "R. K. Girdhari",
  productName: "CapSnap",
  slug: "nocapsnap",
  androidApplicationId: "com.hammurabicoding.nocapsnap",
} as const;

export const ROLES = ["admin", "manager", "chef", "server"] as const;
export type StaffRole = (typeof ROLES)[number];

export const PHOTO_STATUS = ["captured", "sent", "viewed", "reviewed"] as const;
export type PhotoStatus = (typeof PHOTO_STATUS)[number];
