# TODO

- Fix climb column not being hooked up on the flights page
- Make settings persistent by user, if they are logged in.
- Create a club_memberships table that contains:
-   id (uuid)
-   user_id (uuid, not null, foreign key to users)
-   club_id (uuid, not null, foreign key to clubs)
-   is_admin (boolean, default false)
-   created_by_user (uuid, null, foreign key to users)
-   created_at (timestamp, not null, default current timestamp)
-   updated_at (timestamp, not null, default current timestamp)
-   there should be a unique index on user_id and club_id
-

## Features

- /clubs/[id]/duty-manager

Create this page. Require a login. Require that the user logged in be an administrator for this club. Feature a list of aircraft assigned to that club. For the current day (i.e., since midnight local time) display all takeoffs and landings of flights.
