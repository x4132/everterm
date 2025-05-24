import { useParams, useRouter } from "@tanstack/react-router";
import { Button } from "./ui/button";

export default function InvalidID() {
  const { itemId } = useParams({ strict: false });
  const { history } = useRouter();

  return (
    <div className="p-2">
      <h1>Invalid ID "{itemId}"</h1>
      <Button variant={"link"} className="p-0" onClick={() => history.back()}>
        Go Back
      </Button>
    </div>
  );
}
