-- Test data for get_user_names_with_email function

-- Create test macro users
INSERT INTO macro_user (id, username, email, stripe_customer_id)
VALUES
    ('11111111-1111-1111-1111-111111111111', 'test_user_1', 'user_profile_1@macro.com', 'cus_test1'),
    ('22222222-2222-2222-2222-222222222222', 'test_user_2', 'user_profile_2@macro.com', 'cus_test2'),
    ('33333333-3333-3333-3333-333333333333', 'test_user_3', 'user_profile_3@macro.com', 'cus_test3'),
    ('44444444-4444-4444-4444-444444444444', 'test_user_4', 'user_profile_4@macro.com', 'cus_test4'),
    ('55555555-5555-5555-5555-555555555555', 'test_user_5', 'user_profile_5@macro.com', 'cus_test5'),
    ('66666666-6666-6666-6666-666666666666', 'contact', 'contact@example.com', 'cus_test6');

-- Create macro user info with names
INSERT INTO macro_user_info (macro_user_id, first_name, last_name)
VALUES
    ('11111111-1111-1111-1111-111111111111', 'JohnMacroContact', 'DoeMacroContact'),
    ('22222222-2222-2222-2222-222222222222', 'JaneMacroContact', 'SmithMacroContact'),
    ('33333333-3333-3333-3333-333333333333', 'N/A', 'N/A'), -- User with N/A names
    ('44444444-4444-4444-4444-444444444444', 'OnlyFirstMacro', 'N/A'), -- Either populated => use Macro for BOTH; last should be NULL
    ('55555555-5555-5555-5555-555555555555', 'N/A', 'OnlyLastMacro');  -- Either populated => use Macro for BOTH; first should be NULL

-- Create User (profile) entries
INSERT INTO "User" (id, email, macro_user_id, "organizationId", "hasChromeExt", "tutorialComplete", "hasOnboardingDocuments")
VALUES
    ('macro|user_profile_1@macro.com', 'user_profile_1@macro.com', '11111111-1111-1111-1111-111111111111', NULL, false, false, false),
    ('macro|user_profile_2@macro.com', 'user_profile_2@macro.com', '22222222-2222-2222-2222-222222222222', NULL, false, false, false),
    ('macro|user_profile_3@macro.com', 'user_profile_3@macro.com', '33333333-3333-3333-3333-333333333333', NULL, false, false, false),
    ('macro|user_profile_4@macro.com', 'user_profile_4@macro.com', '44444444-4444-4444-4444-444444444444', NULL, false, false, false),
    ('macro|user_profile_5@macro.com', 'user_profile_5@macro.com', '55555555-5555-5555-5555-555555555555', NULL, false, false, false),
    ('macro|contact@example.com', 'contact@example.com', '66666666-6666-6666-6666-666666666666', NULL, false, false, false);
