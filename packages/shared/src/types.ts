import type { PhotoStatus, StaffRole } from "./constants";

export type UUID = string;

export interface Organization {
  id: UUID;
  name: string;
  slug: string;
  logoUrl?: string;
}

export interface Location {
  id: UUID;
  organizationId: UUID;
  name: string;
  address: string;
  timezone: string;
  isActive: boolean;
}

export interface Staff {
  id: UUID;
  locationId: UUID;
  email: string;
  firstName: string;
  lastName: string;
  role: StaffRole;
}

export interface MenuItem {
  id: UUID;
  locationId: UUID;
  name: string;
  category?: string;
  price?: number;
  isActive: boolean;
}

export interface Photo {
  id: UUID;
  locationId: UUID;
  staffId: UUID;
  menuItemId: UUID;
  originalUrl: string;
  watermarkedUrl: string;
  thumbnailUrl: string;
  tableNumber?: string;
  qrCodeToken: string;
  qrCodeUrl: string;
  status: PhotoStatus;
  capturedAt: string;
}

export interface Review {
  id: UUID;
  photoId: UUID;
  locationId: UUID;
  rating: 1 | 2 | 3 | 4 | 5;
  comment?: string;
  customerName?: string;
}

export interface CapturePhotoRequest {
  locationId: UUID;
  menuItemId: UUID;
  tableNumber?: string;
  imageBase64: string;
}

export interface CapturePhotoResponse {
  photo: Photo;
}

export interface SubmitReviewRequest {
  token: string;
  rating: 1 | 2 | 3 | 4 | 5;
  comment?: string;
  customerName?: string;
}

export interface ApiError {
  error: string;
  code?: string;
}
