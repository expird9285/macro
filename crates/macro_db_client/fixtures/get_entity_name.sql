INSERT INTO macro_user (id, email, stripe_customer_id, username) VALUES
('11111111-1111-1111-1111-111111111111', 'user@user.com', 'cus_1234', 'user');

INSERT INTO "User" ("id", "email", "name", "stripeCustomerId", macro_user_id) VALUES
('macro|user@user.com', 'user@user.com', 'User', 'cus_1234', '11111111-1111-1111-1111-111111111111');

INSERT INTO "Document" ("id", "name", "fileType", "owner", "createdAt", "updatedAt")
VALUES
    ('11111111-1111-1111-1111-111111111111', 'document', 'pdf', 'macro|user@user.com', '2019-10-16 00:00:00', '2019-10-16 00:00:00');

INSERT INTO "Chat" ("id", "name", "userId", "createdAt", "updatedAt")
VALUES
    ('11111111-1111-1111-1111-111111111111', 'chat', 'macro|user@user.com', '2019-10-16 01:01:00', '2019-10-16 01:01:00');

INSERT INTO comms_channels (id, name, channel_type, org_id, owner_id)
VALUES ('11111111-1111-1111-1111-111111111111', 'channel', 'public', NULL, 'macro|user@user.com');
