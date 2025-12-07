import { Container, AspectRatio, Loader } from '@mantine/core';

export const SuspenseLoader = () => (
  <Container size="xs">
    <AspectRatio ratio={1}>
      <Loader />
    </AspectRatio>
  </Container>
);
