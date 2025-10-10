# TODO

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
- aircraft model details on the devices/[id] page looks terrible on desktop, no need to have every data point on its own line.
- On the login page, the response isn't necessarily json: "Invalid credentials" is the whole response. Make it json just like the other /data endpoints. Also log the reason why the authentication failed (email not verified? wrong password?) But do not display that to the user.
- The receivers list should also have a "near me" feature for location search.

## Features

- /clubs/[id]/duty-manager

Create this page. Require a login. Require that the user logged in be an administrator for this club. Feature a list of aircraft assigned to that club. For the current day (i.e., since midnight local time) display all takeoffs and landings of flights.

- /flights/[id] - add altitude chart below the map. when hovering over (or touching, on mobile) a point (derived from a fix) on the map, display the altitude (both MSL and AGL) at the nearest fix.
