"use client";

import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Separator } from "@/components/ui/separator";
import { UserCircle, Phone, Mail, KeyRound } from "lucide-react";

export default function AccountSettingsPage() {
  return (
    <div className="p-6 space-y-8 max-w-2xl">
      <div>
        <h1 className="text-2xl font-bold">Account Settings</h1>
        <p className="text-sm text-muted-foreground mt-1">
          Manage your profile, contact info, and security preferences.
        </p>
      </div>

      {/* Profile */}
      <section className="space-y-4">
        <div className="flex items-center gap-2">
          <UserCircle className="h-5 w-5 text-muted-foreground" />
          <h2 className="text-lg font-semibold">Profile</h2>
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="space-y-2">
            <Label htmlFor="display-name">Display Name</Label>
            <Input id="display-name" placeholder="Your name" disabled />
          </div>
          <div className="space-y-2">
            <Label htmlFor="title">Title / Role</Label>
            <Input id="title" placeholder="e.g. Editor, Reporter" disabled />
          </div>
        </div>
      </section>

      <Separator />

      {/* Contact */}
      <section className="space-y-4">
        <div className="flex items-center gap-2">
          <Phone className="h-5 w-5 text-muted-foreground" />
          <h2 className="text-lg font-semibold">Contact Information</h2>
        </div>
        <div className="grid gap-4 sm:grid-cols-2">
          <div className="space-y-2">
            <Label htmlFor="phone">Phone Number</Label>
            <Input id="phone" type="tel" placeholder="+1 (555) 000-0000" disabled />
            <p className="text-xs text-muted-foreground">
              Used for login verification (OTP).
            </p>
          </div>
          <div className="space-y-2">
            <Label htmlFor="email">Email Address</Label>
            <Input id="email" type="email" placeholder="you@example.com" disabled />
          </div>
        </div>
      </section>

      <Separator />

      {/* Security */}
      <section className="space-y-4">
        <div className="flex items-center gap-2">
          <KeyRound className="h-5 w-5 text-muted-foreground" />
          <h2 className="text-lg font-semibold">Security</h2>
        </div>
        <div className="space-y-3">
          <div className="flex items-center justify-between rounded-lg border border-border p-4">
            <div>
              <p className="text-sm font-medium">Phone Verification</p>
              <p className="text-xs text-muted-foreground">
                Your phone number is your primary authentication method.
              </p>
            </div>
            <Button variant="outline" size="sm" disabled>
              Change Number
            </Button>
          </div>
          <div className="flex items-center justify-between rounded-lg border border-border p-4">
            <div>
              <p className="text-sm font-medium">Active Sessions</p>
              <p className="text-xs text-muted-foreground">
                Manage your active login sessions.
              </p>
            </div>
            <Button variant="outline" size="sm" disabled>
              View Sessions
            </Button>
          </div>
        </div>
      </section>

      <Separator />

      <div className="flex justify-end">
        <Button disabled>Save Changes</Button>
      </div>

      <p className="text-xs text-muted-foreground">
        Account settings are not yet connected to the API. All fields are read-only placeholders.
      </p>
    </div>
  );
}
