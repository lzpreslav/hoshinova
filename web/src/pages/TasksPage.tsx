import {
  Anchor,
  AspectRatio,
  Badge,
  Button,
  Card,
  Container,
  Group,
  Image,
  Select,
  SimpleGrid,
  Stack,
  Table,
  Text,
  TextInput,
  Title,
  Box,
} from '@mantine/core';
import React from 'react';
import { stateString, useMutateCreateTask, useQueryTasks } from '../api/tasks';
import { TaskWithStatus } from '../bindings/TaskWithStatus';
import { SuspenseLoader } from '../shared/SuspenseLoader';
import { IconPlus } from '@tabler/icons-react';
import { closeAllModals, openModal } from '@mantine/modals';
import { showNotification } from '@mantine/notifications';
import { useQueryConfig } from '../api/config';
import { YTAState } from '../bindings/YTAState';

const SleepingPanda = React.lazy(() => import('../lotties/SleepingPanda'));

const TaskStateBadge = ({ state }: { state: YTAState }) => (
  <Badge
    color={
      state === 'Recording'
        ? 'green'
        : state === 'Finished'
        ? 'blue'
        : state === 'Muxing'
        ? 'yellow'
        : state === 'Idle' || state === 'AlreadyProcessed' || state === 'Ended'
        ? 'gray'
        : state === 'Interrupted' || state === 'Errored'
        ? 'red'
        : 'violet'
    }
    variant="filled"
  >
    {stateString(state)}
  </Badge>
);

const rowElements = ({ task, status }: TaskWithStatus) => [
  <Image width={160} height={90} radius="md" src={task.video_picture} />,
  <>
    <Anchor
      style={{ display: 'block' }}
      href={'https://www.youtube.com/watch?v=' + task.video_id}
    >
      {task.title}
    </Anchor>
    <Anchor
      color="dimmed"
      style={{ display: 'block' }}
      href={'https://www.youtube.com/channel/' + task.channel_id}
    >
      {task.channel_name}
    </Anchor>
  </>,
  <TaskStateBadge state={status.state} />,
  <>
    {status.total_size === null ? (
      'None'
    ) : (
      <>
        V: {status.video_fragments || '?'} / A: {status.audio_fragments || '?'}{' '}
        / DL: {status.total_size || '?'}
      </>
    )}
  </>,
];

const AddVideoModal = () => {
  const qConfig = useQueryConfig();
  const mCreateTask = useMutateCreateTask();

  const [videoURL, setVideoURL] = React.useState('');
  const [destPaths, setDestPaths] = React.useState<
    { value: string; label: string }[]
  >([]);
  const [destPath, setDestPath] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (!qConfig.data) return;
    const outPaths = new Set((qConfig.data as any).channel.map((ch: any) => ch.outpath));
    if (outPaths.size > destPaths.length) {
      setDestPaths(
        Array.from(outPaths).map((path) => ({ value: path as string, label: path as string }))
      );
    }
  }, [qConfig, destPaths]);

  const addVideo = () => {
    if (!destPath || !videoURL) return;

    mCreateTask.mutate(
      {
        video_url: videoURL,
        output_directory: destPath,
      },
      {
        onSuccess() {
          showNotification({
            message: 'Video added',
            color: 'green',
          });
          closeAllModals();
        },
        async onError(err) {
          let message = '';
          if (err instanceof Response) message = await err.text();
          showNotification({
            title: 'Error adding video',
            message,
            color: 'red',
          });
          console.error(err);
        },
      }
    );
  };

  return (
    <Stack gap="md">
      <TextInput
        label="Video URL"
        placeholder="https://www.youtube.com/watch?v=..."
        data-autofocus
        value={videoURL}
        onChange={(e) => setVideoURL(e.target.value)}
      />
      <Select
        label="Destination Path"
        data={destPaths}
        placeholder="Select a destination path"
        searchable
        onChange={(e: string | null) => setDestPath(e)}
      />
      <Button fullWidth onClick={addVideo}>
        Add
      </Button>
    </Stack>
  );
};

const TasksPage = () => {
  const qTasks = useQueryTasks();
  const tasks = qTasks.data || [];

  const handleAddVideo = () => {
    openModal({
      title: 'Add Video',
      children: <AddVideoModal />,
    });
  };

  if (qTasks.isLoading && !qTasks.data) return <SuspenseLoader />;
  if (tasks.length < 1)
    return (
      <Container size="xs">
        <AspectRatio ratio={1}>
          <React.Suspense fallback={<div />}>
            <SleepingPanda />
          </React.Suspense>
        </AspectRatio>
        <Title>Crickets...</Title>
        <Text>
          There's nothing here yet. Maybe add some more channels to spice things
          up!
        </Text>
        <Group py="md">
          <Button leftSection={<IconPlus size={18} />} onClick={handleAddVideo}>
            Add video
          </Button>
        </Group>
      </Container>
    );

  return (
    <Stack p="md" gap="md">
            <Group>
        <Button leftSection={<IconPlus size={18} />} onClick={handleAddVideo}>
          Add video
        </Button>
      </Group>
      <Box hiddenFrom="md">
        <Table>
          <thead>
            <tr>
              <th>Thumbnail</th>
              <th>Video</th>
              <th>Status</th>
              <th>Progress</th>
            </tr>
          </thead>
          <tbody>
            {tasks.map(({ task, status }) => (
              <tr key={task.video_id}>
                {rowElements({ task, status }).map((row, idx) => (
                  <td key={idx}>{row}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </Table>
      </Box>
      <Box visibleFrom="md">
        <SimpleGrid
          spacing="md"
          cols={{ base: 1, sm: 2, md: 3 }}
        >
          {tasks.map(({ task, status }) => {
            const [_, title, state, progres] = rowElements({ task, status });
            return (
              <Card key={task.video_id}>
                <Card.Section>
                  <AspectRatio ratio={16 / 9}>
                    <Image fit="cover" width="100%" src={task.video_picture} />
                  </AspectRatio>
                </Card.Section>
                <Stack my="lg" gap="md">
                  <div>{title}</div>
                  {state}
                  <div>
                    <Text fw="bold">Progress</Text>
                    {progres}
                  </div>
                </Stack>
              </Card>
            );
          })}
        </SimpleGrid>
      </Box>
    </Stack>
  );
};

export default TasksPage;
